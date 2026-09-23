//! Local-auth Tauri commands.
//!
//! The public surface (no `require_auth` call): `has_users`, `setup_admin`,
//! `login`, `logout`, `is_logged_in`, `current_user`. Every other command in
//! the app calls `state.require_auth().await?` first, and user administration
//! additionally requires `Permission::ManageUsers`.

use tauri::State;

use warfarin_core::models::auth::{
  AuthError, CreateUserInput, LoginInput, ManagedUser, Permission, PublicUser, SetupAdminInput,
  UserRole,
};
use warfarin_db::auth_service;
use warfarin_db::sqlite::AppState;

use crate::session_store;

/// Returns `true` when at least one user exists in the `users` table.
///
/// Used by the frontend to decide between the first-time setup screen and
/// the login screen.
#[tauri::command]
pub async fn has_users(state: State<'_, AppState>) -> Result<bool, String> {
  Ok(auth_service::has_users(&state.pool).await)
}

/// Returns the most recent successful login on this machine, or `None`.
///
/// Machine-wide on purpose: it never reveals whether a given username exists.
#[tauri::command]
pub async fn get_last_login_hint(state: State<'_, AppState>) -> Result<Option<String>, String> {
  Ok(auth_service::last_login_hint(&state.pool).await)
}

/// Creates the first administrator account. Refuses to run when any user
/// already exists (the frontend also gates the setup screen on `has_users`).
#[tauri::command]
pub async fn setup_admin(
  input: SetupAdminInput,
  state: State<'_, AppState>,
) -> Result<PublicUser, String> {
  let auth = auth_service::setup_admin(&state.pool, &state.auth_session, &state.machine_id, input)
    .await
    .map_err(map_auth_error)?;
  persist_token(&auth.token);
  Ok(auth.user)
}

/// Authenticates `username`/`password` against the `users` table and
/// populates the in-memory session slot on success.
///
/// All error variants are mapped to a generic, user-safe Thai message so the
/// frontend never learns whether the username exists.
#[tauri::command]
pub async fn login(input: LoginInput, state: State<'_, AppState>) -> Result<PublicUser, String> {
  let auth = auth_service::login(&state.pool, &state.auth_session, &state.machine_id, input)
    .await
    .map_err(map_auth_error)?;
  persist_token(&auth.token);
  Ok(auth.user)
}

/// Clears the in-memory session and its keychain token. Always succeeds;
/// safe to call repeatedly.
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
  auth_service::logout(&state.pool, &state.auth_session).await;
  session_store::clear_token();
  Ok(())
}

/// Stores the token best-effort: a machine without a usable keychain still
/// gets a working in-memory session, it just will not survive a restart.
fn persist_token(token: &str) {
  if let Err(e) = session_store::store_token(token) {
    eprintln!("[auth] session token not persisted: {e}");
  }
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

/// Lists every account for the admin user management screen.
#[tauri::command]
pub async fn list_users(state: State<'_, AppState>) -> Result<Vec<ManagedUser>, String> {
  state.require_permission(Permission::ManageUsers).await?;
  auth_service::list_users(&state.pool)
    .await
    .map_err(map_auth_error)
}

/// Creates a staff account with an explicit role.
#[tauri::command]
pub async fn create_user(
  input: CreateUserInput,
  state: State<'_, AppState>,
) -> Result<ManagedUser, String> {
  let actor = state.require_permission(Permission::ManageUsers).await?;
  auth_service::create_user(&state.pool, &actor, input)
    .await
    .map_err(map_auth_error)
}

/// Replaces a user's password and clears their lockout state.
#[tauri::command]
pub async fn reset_user_password(
  user_id: i64,
  new_password: String,
  state: State<'_, AppState>,
) -> Result<(), String> {
  let actor = state.require_permission(Permission::ManageUsers).await?;
  auth_service::reset_password(&state.pool, &actor, user_id, &new_password)
    .await
    .map_err(map_auth_error)
}

/// Changes another user's role.
#[tauri::command]
pub async fn set_user_role(
  user_id: i64,
  role: UserRole,
  state: State<'_, AppState>,
) -> Result<(), String> {
  let actor = state.require_permission(Permission::ManageUsers).await?;
  auth_service::set_role(&state.pool, &actor, user_id, role)
    .await
    .map_err(map_auth_error)
}

/// Enables or disables an account.
#[tauri::command]
pub async fn set_user_active(
  user_id: i64,
  active: bool,
  state: State<'_, AppState>,
) -> Result<(), String> {
  let actor = state.require_permission(Permission::ManageUsers).await?;
  auth_service::set_active(&state.pool, &actor, user_id, active)
    .await
    .map_err(map_auth_error)
}

fn map_auth_error(e: AuthError) -> String {
  match e {
    AuthError::InvalidCredentials => "ชื่อผู้ใช้หรือรหัสผ่านไม่ถูกต้อง".to_string(),
    AuthError::AccountLocked => "บัญชีถูกล็อกชั่วคราว กรุณาลองใหม่ในอีก 15 นาที".to_string(),
    AuthError::AccountInactive => "บัญชีนี้ถูกระงับการใช้งาน".to_string(),
    AuthError::SetupUnavailable => "ไม่สามารถสร้างผู้ดูแลระบบเพิ่มได้ ระบบมีผู้ใช้งานอยู่แล้ว".to_string(),
    AuthError::UsernameTaken => "ชื่อผู้ใช้นี้ถูกใช้แล้ว".to_string(),
    AuthError::UserNotFound => "ไม่พบผู้ใช้ที่ระบุ".to_string(),
    AuthError::LastAdmin => "ต้องมีผู้ดูแลระบบที่ใช้งานอยู่อย่างน้อย 1 คน".to_string(),
    AuthError::SelfModification => "ไม่สามารถแก้ไขบัญชีของตนเองในหน้านี้ได้".to_string(),
    AuthError::Validation(msg) => msg,
    AuthError::Database(msg) => {
      eprintln!("[auth] database error surfaced to UI: {msg}");
      "เกิดข้อผิดพลาด กรุณาลองใหม่อีกครั้ง".to_string()
    }
  }
}
