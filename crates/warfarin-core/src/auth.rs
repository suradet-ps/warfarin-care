//! Pure local-auth helpers: password hashing, credential validation, and
//! rate-limit constants.
//!
//! This module has no I/O and no runtime coupling, so the same functions are
//! used by the data layer (during `INSERT` / `verify_password`) and by any
//! future test harness.

#[cfg(miri)]
use argon2::{Algorithm, Params, Version};
use argon2::{
  Argon2,
  password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

/// Minimum length for a username.
pub const MIN_USERNAME_LEN: usize = 3;
/// Maximum length for a username (bounds hashing work and matches UI limits).
pub const MAX_USERNAME_LEN: usize = 32;
/// Minimum length for a user-chosen password.
pub const MIN_PASSWORD_LEN: usize = 8;
/// Maximum length for a user-chosen password (prevents Argon2id `DoS`).
pub const MAX_PASSWORD_LEN: usize = 128;
/// Consecutive failed-login count that triggers a temporary lockout.
pub const MAX_FAILED_ATTEMPTS: u32 = 5;
/// How long a locked account remains locked, in minutes.
pub const LOCKOUT_DURATION_MIN: u32 = 15;

/// Returns the lockout expiry as `now + LOCKOUT_DURATION_MIN` in RFC 3339 UTC.
///
/// Exposed so the service layer can persist the same value it just compared
/// against, without recomputing.
#[must_use]
pub fn lockout_until_now() -> String {
  let now = chrono::Utc::now() + chrono::Duration::minutes(i64::from(LOCKOUT_DURATION_MIN));
  now.to_rfc3339()
}

/// Returns the current UTC time as an RFC 3339 string, used for `created_at` /
/// `updated_at` columns on the `users` table.
#[must_use]
pub fn now_rfc3339() -> String {
  chrono::Utc::now().to_rfc3339()
}

/// Validates a username against the local-auth rules.
///
/// Rules:
/// - non-empty after trimming,
/// - between [`MIN_USERNAME_LEN`] and [`MAX_USERNAME_LEN`] characters,
/// - only ASCII letters, digits, `_`, `-`, or `.`.
///
/// # Errors
///
/// Returns an `Err` with a human-readable, user-safe message when the input
/// fails any rule. The string is suitable for surfacing directly in the UI.
pub fn validate_username(input: &str) -> Result<(), String> {
  let trimmed = input.trim();
  if trimmed.is_empty() {
    return Err("กรุณากรอกชื่อผู้ใช้".to_string());
  }
  if trimmed.len() < MIN_USERNAME_LEN {
    return Err(format!("ชื่อผู้ใช้ต้องมีอย่างน้อย {MIN_USERNAME_LEN} ตัวอักษร"));
  }
  if trimmed.len() > MAX_USERNAME_LEN {
    return Err(format!("ชื่อผู้ใช้ต้องไม่เกิน {MAX_USERNAME_LEN} ตัวอักษร"));
  }
  if !trimmed
    .chars()
    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
  {
    return Err("ชื่อผู้ใช้ใช้ได้เฉพาะตัวอักษร ตัวเลข _ - . เท่านั้น".to_string());
  }
  Ok(())
}

/// Validates a candidate password against the local-auth strength rules.
///
/// Rules:
/// - length in [`MIN_PASSWORD_LEN`, `MAX_PASSWORD_LEN`],
/// - contains at least one ASCII letter and one ASCII digit.
///
/// # Errors
///
/// Returns an `Err` with a human-readable, user-safe message when the input
/// fails any rule.
pub fn validate_password_strength(input: &str) -> Result<(), String> {
  if input.chars().count() < MIN_PASSWORD_LEN {
    return Err(format!("รหัสผ่านต้องมีอย่างน้อย {MIN_PASSWORD_LEN} ตัวอักษร"));
  }
  if input.chars().count() > MAX_PASSWORD_LEN {
    return Err(format!("รหัสผ่านต้องไม่เกิน {MAX_PASSWORD_LEN} ตัวอักษร"));
  }
  if !input.chars().any(|c| c.is_ascii_alphabetic()) {
    return Err("รหัสผ่านต้องมีตัวอักษรอย่างน้อย 1 ตัว".to_string());
  }
  if !input.chars().any(|c| c.is_ascii_digit()) {
    return Err("รหัสผ่านต้องมีตัวเลขอย่างน้อย 1 ตัว".to_string());
  }
  Ok(())
}

fn argon2_hasher() -> Argon2<'static> {
  #[cfg(miri)]
  {
    Argon2::new(
      Algorithm::Argon2id,
      Version::V0x13,
      Params::new(8, 1, 1, None).expect("Miri Argon2 params should be valid"),
    )
  }
  #[cfg(not(miri))]
  {
    Argon2::default()
  }
}

/// Hashes a plaintext password with Argon2id using a fresh random salt.
///
/// Returns a PHC-formatted string (`$argon2id$...`) that can be stored in
/// `users.password_hash` and later passed to [`verify_password`].
///
/// # Errors
///
/// Returns an `Err` string if the underlying Argon2id implementation rejects
/// the input (e.g. empty password) or fails to produce a hash.
pub fn hash_password(plaintext: &str) -> Result<String, String> {
  if plaintext.is_empty() {
    return Err("password is empty".to_string());
  }
  let salt = SaltString::generate(&mut OsRng);
  let hasher = argon2_hasher();
  let hash = hasher
    .hash_password(plaintext.as_bytes(), &salt)
    .map_err(|e| format!("password hash failed: {e}"))?;
  Ok(hash.to_string())
}

/// Verifies a plaintext password against a stored PHC-formatted Argon2id hash.
///
/// Returns `Ok(true)` on a match, `Ok(false)` on a clean mismatch, and an
/// `Err` only when the stored hash is malformed (treat as a hard failure - do
/// not silently accept).
///
/// # Errors
///
/// Returns `Err` when the stored hash cannot be parsed.
pub fn verify_password(plaintext: &str, phc_hash: &str) -> Result<bool, String> {
  let parsed = PasswordHash::new(phc_hash).map_err(|e| format!("invalid stored hash: {e}"))?;
  match argon2_hasher().verify_password(plaintext.as_bytes(), &parsed) {
    Ok(()) => Ok(true),
    Err(argon2::password_hash::Error::Password) => Ok(false),
    Err(e) => Err(format!("password verify failed: {e}")),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validate_username_accepts_valid_inputs() {
    assert!(validate_username("admin").is_ok());
    assert!(validate_username("user.name_1-2").is_ok());
    assert!(validate_username("a1.").is_ok());
  }

  #[test]
  fn validate_username_rejects_short() {
    let err = validate_username("ab").unwrap_err();
    assert!(err.contains("อย่างน้อย"));
  }

  #[test]
  fn validate_username_rejects_empty() {
    assert!(validate_username("   ").is_err());
  }

  #[test]
  fn validate_username_rejects_too_long() {
    let long = "a".repeat(MAX_USERNAME_LEN + 1);
    assert!(validate_username(&long).is_err());
  }

  #[test]
  fn validate_username_rejects_invalid_chars() {
    assert!(validate_username("user name").is_err());
    assert!(validate_username("user!").is_err());
    assert!(validate_username("user@host").is_err());
  }

  #[test]
  fn validate_password_rejects_short() {
    assert!(validate_password_strength("Abc1").is_err());
  }

  #[test]
  fn validate_password_rejects_no_letter() {
    assert!(validate_password_strength("12345678").is_err());
  }

  #[test]
  fn validate_password_rejects_no_digit() {
    assert!(validate_password_strength("abcdefgh").is_err());
  }

  #[test]
  fn validate_password_rejects_too_long() {
    let p = format!("A1{}", "a".repeat(MAX_PASSWORD_LEN));
    assert!(validate_password_strength(&p).is_err());
  }

  #[test]
  fn validate_password_accepts_valid() {
    assert!(validate_password_strength("Password1").is_ok());
  }

  #[test]
  fn hash_password_round_trips() {
    let h = hash_password("Password1").unwrap();
    assert!(h.starts_with("$argon2id$"));
    assert!(verify_password("Password1", &h).unwrap());
    assert!(!verify_password("Password2", &h).unwrap());
  }

  #[test]
  fn hash_password_rejects_empty() {
    assert!(hash_password("").is_err());
  }

  #[test]
  fn verify_password_rejects_malformed_hash() {
    assert!(verify_password("Password1", "not-a-hash").is_err());
  }

  #[test]
  #[cfg_attr(miri, ignore = "requires system clock access")]
  fn lockout_until_now_is_in_the_future() {
    let s = lockout_until_now();
    let parsed = chrono::DateTime::parse_from_rfc3339(&s).unwrap();
    let now = chrono::Utc::now();
    assert!(parsed.with_timezone(&chrono::Utc) > now);
  }
}
