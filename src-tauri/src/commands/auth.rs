//! Local-auth Tauri commands.
//!
//! These are the only IPC entry points the Vue frontend has for the auth
//! subsystem. Six commands, all in the public surface (no `require_auth`
//! call): `has_users`, `setup_admin`, `login`, `logout`, `is_logged_in`,
//! `current_user`. Every other command in the app calls
//! `state.require_auth().await?` first.

use tauri::State;

use warfarin_core::models::auth::{AuthError, LoginInput, PublicUser, SetupAdminInput};
use warfarin_db::auth_service;
use warfarin_db::sqlite::AppState;

/// Returns `true` when at least one user exists in the `users` table.
///
/// Used by the frontend to decide between the first-time setup screen and
/// the login screen.
#[tauri::command]
pub async fn has_users(state: State<'_, AppState>) -> Result<bool, String> {
  Ok(auth_service::has_users(&state.pool).await)
}

/// Creates the first administrator account. Refuses to run when any user
/// already exists (the frontend also gates the setup screen on `has_users`).
#[tauri::command]
pub async fn setup_admin(
  input: SetupAdminInput,
  state: State<'_, AppState>,
) -> Result<PublicUser, String> {
  auth_service::setup_admin(&state.pool, &state.auth_session, input)
    .await
    .map_err(map_auth_error)
}

/// Authenticates `username`/`password` against the `users` table and
/// populates the in-memory session slot on success.
///
/// All error variants are mapped to a generic, user-safe Thai message so the
/// frontend never learns whether the username exists.
#[tauri::command]
pub async fn login(input: LoginInput, state: State<'_, AppState>) -> Result<PublicUser, String> {
  auth_service::login(&state.pool, &state.auth_session, input)
    .await
    .map_err(map_auth_error)
}

/// Clears the in-memory session. Always succeeds; safe to call repeatedly.
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
  auth_service::logout(&state.pool, &state.auth_session).await;
  Ok(())
}

/// Returns `true` when a session is currently held in memory.
#[tauri::command]
pub async fn is_logged_in(state: State<'_, AppState>) -> Result<bool, String> {
  Ok(state.is_authenticated().await)
}

/// Returns the public view of the current user, or `None` if not logged in.
#[tauri::command]
pub async fn current_user(state: State<'_, AppState>) -> Result<Option<PublicUser>, String> {
  Ok(state.current_user().await)
}

fn map_auth_error(e: AuthError) -> String {
  match e {
    AuthError::InvalidCredentials => "ชื่อผู้ใช้หรือรหัสผ่านไม่ถูกต้อง".to_string(),
    AuthError::AccountLocked => "บัญชีถูกล็อกชั่วคราว กรุณาลองใหม่ในอีก 15 นาที".to_string(),
    AuthError::AccountInactive => "บัญชีนี้ถูกระงับการใช้งาน".to_string(),
    AuthError::SetupUnavailable => "ไม่สามารถสร้างผู้ดูแลระบบเพิ่มได้ ระบบมีผู้ใช้งานอยู่แล้ว".to_string(),
    AuthError::UsernameTaken => "ชื่อผู้ใช้นี้ถูกใช้แล้ว".to_string(),
    AuthError::Validation(msg) => msg,
    AuthError::Database(msg) => {
      eprintln!("[auth] database error surfaced to UI: {msg}");
      "เกิดข้อผิดพลาด กรุณาลองใหม่อีกครั้ง".to_string()
    }
  }
}
