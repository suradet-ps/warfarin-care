use aes_gcm::{
  Aes256Gcm, Nonce,
  aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hkdf::Hkdf;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const APP_SALT: &[u8] = b"warfarin-care-aes-v1";
const HKDF_INFO: &[u8] = b"warfarin-care-anon-key-v1";
const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EncryptedData {
  pub nonce: String,
  pub ciphertext: String,
}

#[must_use]
pub fn generate_key() -> [u8; KEY_SIZE] {
  let mut key = [0u8; KEY_SIZE];
  rand::rng().fill_bytes(&mut key);
  key
}

fn derive_key(machine_id: &str) -> [u8; KEY_SIZE] {
  let hk = Hkdf::<Sha256>::new(Some(APP_SALT), machine_id.as_bytes());
  let mut key = [0u8; KEY_SIZE];
  hk.expand(HKDF_INFO, &mut key)
    .expect("HKDF expand should not fail");
  key
}

/// Encrypts `plaintext` with `key` and returns the base64-encoded nonce and
/// ciphertext.
///
/// # Errors
///
/// Returns an error string if `key` is not a valid AES-256 key length or the
/// underlying AEAD encryption fails.
pub fn encrypt(plaintext: &str, key: &[u8; KEY_SIZE]) -> Result<EncryptedData, String> {
  let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;

  let mut nonce_bytes = [0u8; NONCE_SIZE];
  rand::rng().fill_bytes(&mut nonce_bytes);
  let nonce = Nonce::from_slice(&nonce_bytes);

  let ciphertext = cipher
    .encrypt(nonce, plaintext.as_bytes())
    .map_err(|e| e.to_string())?;

  Ok(EncryptedData {
    nonce: BASE64.encode(nonce_bytes),
    ciphertext: BASE64.encode(ciphertext),
  })
}

/// Decrypts `encrypted` with `key` and returns the plaintext.
///
/// # Errors
///
/// Returns an error string if `key` is invalid, the base64 payload cannot be
/// decoded, the AEAD decryption fails (wrong key / tampered ciphertext), or
/// the plaintext is not valid UTF-8.
pub fn decrypt(encrypted: &EncryptedData, key: &[u8; KEY_SIZE]) -> Result<String, String> {
  let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;

  let nonce_bytes = BASE64.decode(&encrypted.nonce).map_err(|e| e.to_string())?;
  let ciphertext = BASE64
    .decode(&encrypted.ciphertext)
    .map_err(|e| e.to_string())?;

  let nonce = Nonce::from_slice(&nonce_bytes);

  let plaintext = cipher
    .decrypt(nonce, ciphertext.as_ref())
    .map_err(|e| e.to_string())?;

  String::from_utf8(plaintext).map_err(|e| e.to_string())
}

/// Encrypts a plaintext value with a deterministic key derived from `machine_id`.
///
/// Returns a base64 payload containing `nonce || ciphertext`.
///
/// # Errors
///
/// Returns an error string if key derivation or the AEAD encryption fails.
pub fn encrypt_value(plaintext: &str, machine_id: &str) -> Result<String, String> {
  let key = derive_key(machine_id);
  let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;

  let mut nonce_bytes = [0u8; NONCE_SIZE];
  rand::rng().fill_bytes(&mut nonce_bytes);
  let nonce = Nonce::from_slice(&nonce_bytes);

  let ciphertext = cipher
    .encrypt(nonce, plaintext.as_bytes())
    .map_err(|e| e.to_string())?;

  let mut combined = nonce_bytes.to_vec();
  combined.extend_from_slice(&ciphertext);
  Ok(BASE64.encode(combined))
}

/// Decrypts a base64 payload produced by [`encrypt_value`].
///
/// # Errors
///
/// Returns an error string if the payload is not valid base64, is too short
/// to contain a nonce, the AEAD decryption fails (wrong `machine_id` or
/// tampered data), or the plaintext is not valid UTF-8.
pub fn decrypt_value(encoded: &str, machine_id: &str) -> Result<String, String> {
  let combined = BASE64.decode(encoded).map_err(|e| e.to_string())?;
  if combined.len() <= NONCE_SIZE {
    return Err("Ciphertext too short to contain a valid nonce".to_string());
  }

  let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
  let key = derive_key(machine_id);
  let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
  let nonce = Nonce::from_slice(nonce_bytes);

  let plaintext = cipher
    .decrypt(nonce, ciphertext)
    .map_err(|e| e.to_string())?;

  String::from_utf8(plaintext).map_err(|e| e.to_string())
}

/// Encrypts a serializable value as a JSON envelope (see [`EncryptedData`]).
///
/// # Errors
///
/// Returns an error string if `data` cannot be serialized to JSON or the
/// inner [`encrypt`] call fails.
pub fn encrypt_json<T: Serialize>(data: &T, key: &[u8; KEY_SIZE]) -> Result<String, String> {
  let plaintext = serde_json::to_string(data).map_err(|e| e.to_string())?;
  let encrypted = encrypt(&plaintext, key)?;
  serde_json::to_string(&encrypted).map_err(|e| e.to_string())
}

/// Decrypts a JSON envelope produced by [`encrypt_json`] into `T`.
///
/// # Errors
///
/// Returns an error string if the envelope cannot be parsed, the inner
/// [`decrypt`] call fails, or the plaintext cannot be deserialized into `T`.
pub fn decrypt_json<T: for<'de> serde::Deserialize<'de>>(
  encrypted_json: &str,
  key: &[u8; KEY_SIZE],
) -> Result<T, String> {
  let encrypted: EncryptedData = serde_json::from_str(encrypted_json).map_err(|e| e.to_string())?;
  let plaintext = decrypt(&encrypted, key)?;
  serde_json::from_str(&plaintext).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_encrypt_decrypt() -> Result<(), String> {
    let key = generate_key();
    let plaintext = "my_secret_password";

    let encrypted = encrypt(plaintext, &key)?;
    let decrypted = decrypt(&encrypted, &key)?;

    assert_eq!(plaintext, decrypted);
    Ok(())
  }

  #[test]
  fn test_encrypt_json() -> Result<(), String> {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Config {
      host: String,
      password: String,
    }

    let key = generate_key();
    let config = Config {
      host: "localhost".to_string(),
      password: "secret123".to_string(),
    };

    let encrypted_json = encrypt_json(&config, &key)?;
    let decrypted: Config = decrypt_json(&encrypted_json, &key)?;

    assert_eq!(config, decrypted);
    Ok(())
  }

  #[test]
  fn test_encrypt_value_round_trip() -> Result<(), String> {
    let machine_id = "test-machine-id-1234";
    let plaintext = "supabase-anon-key";

    let encrypted = encrypt_value(plaintext, machine_id)?;
    let decrypted = decrypt_value(&encrypted, machine_id)?;

    assert_eq!(plaintext, decrypted);
    Ok(())
  }

  #[test]
  fn test_decrypt_value_rejects_wrong_machine_id() -> Result<(), String> {
    let encrypted = encrypt_value("secret", "machine-a")?;
    let result = decrypt_value(&encrypted, "machine-b");

    assert!(result.is_err());
    Ok(())
  }
}
