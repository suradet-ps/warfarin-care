//! Settings and `MySQL` connection-test commands.

use std::collections::HashMap;
use std::sync::OnceLock;

use encryptman_keyring::Vault;
use tauri::State;

use warfarin_db::{
  mysql::{DbConfig, test_mysql_connection as db_test_connection},
  sqlite::{AppState, get_all_settings, get_setting, set_setting},
};

const MYSQL_CONFIG_KEY: &str = "mysql_config";
const VAULT_SERVICE: &str = "warfarin-care";

/// In-process cache for the [`Vault`]. The macOS Keychain (and other OS
/// keystores) prompts the user to authorise every read while the app is
/// unsigned. Caching here means the prompt appears at most once per app
/// launch, not on every settings interaction.
static VAULT: OnceLock<Vault> = OnceLock::new();

/// Returns a reference to the cached [`Vault`], initialising it on first call.
fn get_or_create_vault() -> Result<&'static Vault, String> {
  if let Some(vault) = VAULT.get() {
    return Ok(vault);
  }
  let vault = Vault::new(VAULT_SERVICE).map_err(|e| format!("vault init failed: {e}"))?;
  let _ = VAULT.set(vault);
  Ok(VAULT.get().unwrap())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
  state.require_auth().await?;
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
  state.require_auth().await?;
  set_setting(&state.pool, &key, &value)
    .await
    .map_err(|e| e.to_string())
}

/// Persists the `MySQL` config JSON and verifies the connection.
/// Password is encrypted before storage. If the supplied password is empty,
/// the existing stored password is preserved (lets the UI keep the password
/// hidden behind a placeholder without forcing the user to re-type it).
#[tauri::command]
pub async fn test_mysql_connection(
  config: DbConfig,
  state: State<'_, AppState>,
) -> Result<bool, String> {
  state.require_auth().await?;
  let merged = merge_with_stored_password(&state.pool, config).await?;
  let ok = db_test_connection(&merged).await;
  if ok {
    persist_mysql_config(&state.pool, &merged).await?;
  }
  Ok(ok)
}

/// Saves the `MySQL` config without performing a connection test. Used by the
/// "บันทึก" button so users can persist credentials even when `HOSxP` is not
/// currently reachable (e.g. while editing settings off-site). If the supplied
/// password is empty, the existing stored password is preserved.
#[tauri::command]
pub async fn save_mysql_config(config: DbConfig, state: State<'_, AppState>) -> Result<(), String> {
  state.require_auth().await?;
  let merged = merge_with_stored_password(&state.pool, config).await?;
  persist_mysql_config(&state.pool, &merged).await
}

/// If the incoming config has an empty password and a stored config exists,
/// copy the stored password into the incoming config so the rest of the
/// pipeline (test + persist) operates on a complete record.
async fn merge_with_stored_password(
  pool: &sqlx::SqlitePool,
  mut config: DbConfig,
) -> Result<DbConfig, String> {
  if !config.password.is_empty() {
    return Ok(config);
  }
  if let Some(existing) = get_mysql_config_internal(pool).await? {
    config.password = existing.password;
  }
  Ok(config)
}

async fn persist_mysql_config(pool: &sqlx::SqlitePool, config: &DbConfig) -> Result<(), String> {
  let vault = get_or_create_vault()?;
  let json = serde_json::to_string(config).map_err(|e| e.to_string())?;
  let encrypted = vault.encrypt(&json).map_err(|e| e.to_string())?;
  set_setting(pool, MYSQL_CONFIG_KEY, &encrypted)
    .await
    .map_err(|e| e.to_string())
}

/// Returns the `MySQL` config with decrypted password for UI display/editing.
///
/// **Deprecated.** Returns the full `DbConfig` including the password, which is
/// a security concern. Use [`get_mysql_config_status`] for UI display and let
/// the user re-enter the password to update. Kept for backward compatibility
/// with plugins or external scripts that may still depend on it; will be
/// removed in a future release.
#[tauri::command]
pub async fn get_mysql_config_for_ui(
  state: State<'_, AppState>,
) -> Result<Option<DbConfig>, String> {
  state.require_auth().await?;
  let Some(stored) = get_setting(&state.pool, MYSQL_CONFIG_KEY)
    .await
    .map_err(|e| e.to_string())?
  else {
    return Ok(None);
  };

  // Try encrypted format first
  if let Ok(vault) = get_or_create_vault()
    && let Ok(json) = vault.decrypt(&stored)
    && let Ok(config) = serde_json::from_str::<DbConfig>(&json)
  {
    return Ok(Some(config));
  }

  // Fallback: try plaintext (backward compatibility)
  let config: DbConfig = serde_json::from_str(&stored).map_err(|e| e.to_string())?;
  Ok(Some(config))
}

/// Non-secret metadata about the stored `MySQL` config. Safe to call from the UI
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
  state.require_auth().await?;
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
  state.require_auth().await?;
  get_setting(&state.pool, &key)
    .await
    .map_err(|e| e.to_string())
}

/// Internal helper: returns decrypted `MySQL` config for use by other commands.
/// This handles both encrypted (new) and plaintext (legacy) formats.
pub async fn get_mysql_config_internal(
  pool: &sqlx::SqlitePool,
) -> Result<Option<DbConfig>, String> {
  let Some(stored) = get_setting(pool, MYSQL_CONFIG_KEY)
    .await
    .map_err(|e| e.to_string())?
  else {
    return Ok(None);
  };

  // Try encrypted format first
  if let Ok(vault) = get_or_create_vault()
    && let Ok(json) = vault.decrypt(&stored)
    && let Ok(config) = serde_json::from_str::<DbConfig>(&json)
  {
    return Ok(Some(config));
  }

  // Fallback: try plaintext (backward compatibility)
  let config: DbConfig = serde_json::from_str(&stored).map_err(|e| e.to_string())?;
  Ok(Some(config))
}
