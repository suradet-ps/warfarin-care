//! Settings and MySQL connection-test commands.

use std::collections::HashMap;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use tauri::State;

use crate::db::{
  mysql::{DbConfig, test_mysql_connection as db_test_connection},
  sqlite::{AppState, get_all_settings, get_setting, set_setting},
};
use crate::encrypt;

const MYSQL_CONFIG_KEY: &str = "mysql_config";
const ENCRYPTION_KEY_KEY: &str = "encryption_key";
const KEYRING_SERVICE: &str = "warfarin-care";
const KEYRING_USER: &str = "mysql-encryption-key";

fn load_key_from_keyring() -> Result<[u8; 32], String> {
  let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
    .map_err(|e| format!("keyring access failed: {e}"))?;
  let stored = entry
    .get_password()
    .map_err(|e| format!("keyring read failed: {e}"))?;
  let key_bytes = BASE64
    .decode(stored.trim())
    .map_err(|e| format!("keyring value is not valid base64: {e}"))?;
  if key_bytes.len() != 32 {
    return Err(format!(
      "keyring value has wrong length: {} (expected 32)",
      key_bytes.len()
    ));
  }
  let mut key = [0u8; 32];
  key.copy_from_slice(&key_bytes);
  Ok(key)
}

fn store_key_to_keyring(key: &[u8; 32]) -> Result<(), String> {
  let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
    .map_err(|e| format!("keyring access failed: {e}"))?;
  let encoded = BASE64.encode(key);
  entry
    .set_password(&encoded)
    .map_err(|e| format!("keyring write failed: {e}"))
}

/// One-time migration: if a key exists in the SQLite `wf_settings` table
/// (the pre-R-1.5 storage), copy it into the OS keychain and delete the
/// database row so the secret no longer lives on disk in plaintext-equivalent
/// form. If the keychain already has a key, the SQLite row is removed without
/// overwriting.
async fn migrate_key_to_keyring(pool: &sqlx::SqlitePool) -> Result<(), String> {
  let Some(stored_b64) = get_setting(pool, ENCRYPTION_KEY_KEY)
    .await
    .map_err(|e| e.to_string())?
  else {
    return Ok(());
  };
  // Best-effort migration: if the keychain already has a key, prefer the
  // keychain one (it was set by a newer install) and just drop the SQLite row.
  match load_key_from_keyring() {
    Ok(_) => {
      // Keychain already populated — nothing to migrate, just clean up.
    }
    Err(_) => {
      // Keychain empty — try to migrate the SQLite value into it.
      let bytes = BASE64
        .decode(stored_b64.trim())
        .map_err(|e| format!("legacy key is not valid base64: {e}"))?;
      if bytes.len() != 32 {
        return Err(format!(
          "legacy key has wrong length: {} (expected 32)",
          bytes.len()
        ));
      }
      let mut key = [0u8; 32];
      key.copy_from_slice(&bytes);
      store_key_to_keyring(&key)?;
    }
  }
  // Remove the SQLite row regardless of which path we took; the keychain is
  // the new source of truth.
  set_setting(pool, ENCRYPTION_KEY_KEY, "")
    .await
    .map_err(|e| e.to_string())?;
  Ok(())
}

async fn get_or_create_encryption_key(pool: &sqlx::SqlitePool) -> Result<[u8; 32], String> {
  // Try the OS keychain first. If the key isn't there, run the one-time
  // migration from the legacy SQLite row. If still nothing, mint a new key
  // and store it in the keychain.
  if let Ok(key) = load_key_from_keyring() {
    return Ok(key);
  }

  // Best-effort migration of any legacy SQLite-stored key.
  if let Err(e) = migrate_key_to_keyring(pool).await {
    // Migration failure isn't fatal — we can still mint a fresh key. But
    // we should at least log it.
    eprintln!("encryption-key migration: {e}");
  }

  if let Ok(key) = load_key_from_keyring() {
    return Ok(key);
  }

  // No existing key anywhere — mint a new one.
  let key = encrypt::generate_key();
  store_key_to_keyring(&key)?;
  Ok(key)
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
  let pairs = get_all_settings(&state.pool)
    .await
    .map_err(|e| e.to_string())?;
  Ok(pairs.into_iter().collect())
}

#[tauri::command]
pub async fn save_setting(
  key: String,
  value: String,
  state: State<'_, AppState>,
) -> Result<(), String> {
  set_setting(&state.pool, &key, &value)
    .await
    .map_err(|e| e.to_string())
}

/// Persists the MySQL config JSON and verifies the connection.
/// Password is encrypted before storage.
#[tauri::command]
pub async fn test_mysql_connection(
  config: DbConfig,
  state: State<'_, AppState>,
) -> Result<bool, String> {
  let ok = db_test_connection(&config).await;
  if ok {
    let key = get_or_create_encryption_key(&state.pool).await?;
    let encrypted_json = encrypt::encrypt_json(&config, &key)?;
    set_setting(&state.pool, MYSQL_CONFIG_KEY, &encrypted_json)
      .await
      .map_err(|e| e.to_string())?;
  }
  Ok(ok)
}

/// Returns the MySQL config with decrypted password for UI display/editing.
///
/// **Deprecated.** Returns the full DbConfig including the password, which is
/// a security concern. Use [`get_mysql_config_status`] for UI display and let
/// the user re-enter the password to update. Kept for backward compatibility
/// with plugins or external scripts that may still depend on it; will be
/// removed in a future release.
#[tauri::command]
pub async fn get_mysql_config_for_ui(
  state: State<'_, AppState>,
) -> Result<Option<DbConfig>, String> {
  let stored = match get_setting(&state.pool, MYSQL_CONFIG_KEY)
    .await
    .map_err(|e| e.to_string())?
  {
    Some(v) => v,
    None => return Ok(None),
  };

  // Try encrypted format first
  if let Ok(key) = get_or_create_encryption_key(&state.pool).await
    && let Ok(config) = encrypt::decrypt_json::<DbConfig>(&stored, &key)
  {
    return Ok(Some(config));
  }

  // Fallback: try plaintext (backward compatibility)
  let config: DbConfig = serde_json::from_str(&stored).map_err(|e| e.to_string())?;
  Ok(Some(config))
}

/// Non-secret metadata about the stored MySQL config. Safe to call from the UI
/// because the password is never returned. Use the empty-string convention
/// in the form for password: presence of `has_config == true` + non-empty
/// username means a config is stored; the user re-types the password to
/// update.
#[derive(serde::Serialize)]
pub struct MysqlConfigStatus {
  pub has_config: bool,
  pub host: String,
  pub port: u16,
  pub database: String,
  pub username: String,
}

#[tauri::command]
pub async fn get_mysql_config_status(
  state: State<'_, AppState>,
) -> Result<MysqlConfigStatus, String> {
  let cfg = get_mysql_config_internal(&state.pool).await?;
  Ok(match cfg {
    Some(c) => MysqlConfigStatus {
      has_config: true,
      host: c.host,
      port: c.port,
      database: c.database,
      username: c.username,
    },
    None => MysqlConfigStatus {
      has_config: false,
      host: String::new(),
      port: 3306,
      database: String::new(),
      username: String::new(),
    },
  })
}

/// Returns a specific setting value.
#[tauri::command]
pub async fn get_setting_value(
  key: String,
  state: State<'_, AppState>,
) -> Result<Option<String>, String> {
  get_setting(&state.pool, &key)
    .await
    .map_err(|e| e.to_string())
}

/// Internal helper: returns decrypted MySQL config for use by other commands.
/// This handles both encrypted (new) and plaintext (legacy) formats.
pub async fn get_mysql_config_internal(
  pool: &sqlx::SqlitePool,
) -> Result<Option<DbConfig>, String> {
  let stored = match get_setting(pool, MYSQL_CONFIG_KEY)
    .await
    .map_err(|e| e.to_string())?
  {
    Some(v) => v,
    None => return Ok(None),
  };

  // Try encrypted format first
  if let Ok(key) = get_or_create_encryption_key(pool).await
    && let Ok(config) = encrypt::decrypt_json::<DbConfig>(&stored, &key)
  {
    return Ok(Some(config));
  }

  // Fallback: try plaintext (backward compatibility)
  let config: DbConfig = serde_json::from_str(&stored).map_err(|e| e.to_string())?;
  Ok(Some(config))
}
