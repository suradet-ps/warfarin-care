use encryptman::MasterKey;
use serde::{Deserialize, Serialize};

const KEY_SIZE: usize = 32;

/// A well-known application constant used as the master key for
/// `encrypt_value` / `decrypt_value`. The actual secret entropy comes from
/// the `machine_id` passed as the encryptman *context* parameter — matching
/// the same security model as the old HKDF-based derivation where the salt
/// and info were also public constants.
const APP_MASTER_KEY: [u8; KEY_SIZE] = [
  0x77, 0x61, 0x72, 0x66, 0x61, 0x72, 0x69, 0x6e, // warfarin
  0x2d, 0x63, 0x61, 0x72, 0x65, 0x2d, 0x61, 0x70, // -care-ap
  0x70, 0x2d, 0x6d, 0x61, 0x73, 0x74, 0x65, 0x72, // p-master
  0x2d, 0x6b, 0x65, 0x79, 0x21, 0x21, 0x21, 0x21, // -key!!!!
];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EncryptedData {
  pub nonce: String,
  pub ciphertext: String,
}

fn bytes_to_master_key(key: &[u8; KEY_SIZE]) -> MasterKey {
  MasterKey::from_bytes(*key)
}

/// Generate a random 32-byte key suitable for storage in the OS keychain.
#[must_use]
pub fn generate_key() -> [u8; KEY_SIZE] {
  let master = encryptman::generate_master_key();
  *master.as_bytes()
}

/// Encrypts `plaintext` with `key` and returns the encryptman-encoded
/// ciphertext as a base64 string.
///
/// # Errors
///
/// Returns an error string if the underlying AEAD encryption fails.
pub fn encrypt(plaintext: &str, key: &[u8; KEY_SIZE]) -> Result<String, String> {
  let master = bytes_to_master_key(key);
  encryptman::encrypt(&master, plaintext).map_err(|e| e.to_string())
}

/// Decrypts an encryptman-encoded ciphertext produced by [`encrypt`].
///
/// # Errors
///
/// Returns an error string if the base64 payload cannot be decoded, the AEAD
/// decryption fails (wrong key / tampered ciphertext), or the plaintext is
/// not valid UTF-8.
pub fn decrypt(encoded: &str, key: &[u8; KEY_SIZE]) -> Result<String, String> {
  let master = bytes_to_master_key(key);
  encryptman::decrypt(&master, encoded).map_err(|e| e.to_string())
}

/// Encrypts a plaintext value with a deterministic key derived from
/// `machine_id`.
///
/// The `machine_id` is used as the encryptman *context* parameter for HKDF
/// key derivation. Returns a base64-encoded ciphertext string.
///
/// # Errors
///
/// Returns an error string if the AEAD encryption fails.
pub fn encrypt_value(plaintext: &str, machine_id: &str) -> Result<String, String> {
  let master = bytes_to_master_key(&APP_MASTER_KEY);
  encryptman::encrypt_with_context(&master, machine_id, plaintext).map_err(|e| e.to_string())
}

/// Decrypts a ciphertext produced by [`encrypt_value`].
///
/// # Errors
///
/// Returns an error string if decryption fails (wrong `machine_id` or
/// tampered ciphertext), or the plaintext is not valid UTF-8.
pub fn decrypt_value(encoded: &str, machine_id: &str) -> Result<String, String> {
  let master = bytes_to_master_key(&APP_MASTER_KEY);
  encryptman::decrypt_with_context(&master, machine_id, encoded).map_err(|e| e.to_string())
}

/// Encrypts a serializable value as JSON, then encrypts the JSON string.
///
/// # Errors
///
/// Returns an error string if `data` cannot be serialized to JSON or the
/// inner encryption call fails.
pub fn encrypt_json<T: Serialize>(data: &T, key: &[u8; KEY_SIZE]) -> Result<String, String> {
  let plaintext = serde_json::to_string(data).map_err(|e| e.to_string())?;
  encrypt(&plaintext, key)
}

/// Decrypts and deserializes a value produced by [`encrypt_json`] into `T`.
///
/// # Errors
///
/// Returns an error string if the inner decryption call fails, or the
/// plaintext cannot be deserialized into `T`.
pub fn decrypt_json<T: for<'de> Deserialize<'de>>(
  encrypted_json: &str,
  key: &[u8; KEY_SIZE],
) -> Result<T, String> {
  let plaintext = decrypt(encrypted_json, key)?;
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
