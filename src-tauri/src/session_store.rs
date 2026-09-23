//! OS keychain storage for the session token.
//!
//! Only the raw token lives here; the database stores its SHA-256 hash. Debug
//! builds use a separate keychain service so development never reads or
//! overwrites a production session.

use keyring::Entry;

#[cfg(debug_assertions)]
const KEYRING_SERVICE: &str = "warfarin-care.dev";
#[cfg(not(debug_assertions))]
const KEYRING_SERVICE: &str = "warfarin-care";
const TOKEN_ACCOUNT: &str = "session-token";

fn entry() -> Result<Entry, String> {
  Entry::new(KEYRING_SERVICE, TOKEN_ACCOUNT).map_err(|e| format!("keyring unavailable: {e}"))
}

/// Stores the raw session token in the OS keychain.
///
/// # Errors
///
/// Returns a message when the keychain is unavailable; callers treat this as
/// non-fatal and keep the in-memory session.
pub fn store_token(token: &str) -> Result<(), String> {
  entry()?
    .set_password(token)
    .map_err(|e| format!("failed to store session token: {e}"))
}

/// Reads the stored session token, or `None` when absent or unreadable.
#[must_use]
pub fn load_token() -> Option<String> {
  entry().ok()?.get_password().ok()
}

/// Removes the stored token. Missing entries are ignored.
pub fn clear_token() {
  if let Ok(entry) = entry() {
    let _ = entry.delete_credential();
  }
}
