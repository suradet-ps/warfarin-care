//! Domain models for the local-auth subsystem.
//!
//! Only types that need to cross the Tauri IPC boundary or sit on a domain
//! boundary (service ↔ repository) live here. Internal `User` rows with the
//! `password_hash` are kept out of the public DTOs.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// User role, used to gate future role-based actions.
///
/// Persisted as a `&'static str` via [`UserRole::as_str`] / `FromStr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum UserRole {
  Admin,
  User,
}

impl UserRole {
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Admin => "Admin",
      Self::User => "User",
    }
  }
}

impl std::str::FromStr for UserRole {
  type Err = String;
  fn from_str(value: &str) -> Result<Self, Self::Err> {
    match value {
      "Admin" => Ok(Self::Admin),
      "User" => Ok(Self::User),
      other => Err(format!("unknown role: {other}")),
    }
  }
}

/// Internal user record as stored in the `users` table.
///
/// The `password_hash` is **never** returned to the frontend. Use
/// [`PublicUser`] for IPC payloads.
#[derive(Debug, Clone)]
pub struct User {
  pub id: i64,
  pub username: String,
  pub password_hash: String,
  pub role: UserRole,
  pub active: bool,
  pub failed_attempts: u32,
  pub locked_until: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

/// Public user DTO — safe to send to the frontend over Tauri IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
  pub id: i64,
  pub username: String,
  pub role: UserRole,
  pub created_at: String,
}

impl From<&User> for PublicUser {
  fn from(u: &User) -> Self {
    Self {
      id: u.id,
      username: u.username.clone(),
      role: u.role,
      created_at: u.created_at.clone(),
    }
  }
}

/// In-memory session record. Never persisted; lives only in `AppState`.
#[derive(Debug, Clone)]
pub struct AuthSession {
  pub user_id: i64,
  pub username: String,
  pub role: UserRole,
  pub started_at: String,
}

impl AuthSession {
  #[must_use]
  pub fn public_user(&self) -> PublicUser {
    PublicUser {
      id: self.user_id,
      username: self.username.clone(),
      role: self.role,
      created_at: self.started_at.clone(),
    }
  }
}

/// Login form payload (frontend → `login` command).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
  pub username: String,
  pub password: String,
}

/// First-time setup payload (frontend → `setup_admin` command).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupAdminInput {
  pub username: String,
  pub password: String,
}

/// Audit-log event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthEventType {
  LoginSuccess,
  LoginFailed,
  AccountLocked,
  Logout,
  UserCreated,
  SetupCompleted,
}

impl AuthEventType {
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::LoginSuccess => "login_success",
      Self::LoginFailed => "login_failed",
      Self::AccountLocked => "account_locked",
      Self::Logout => "logout",
      Self::UserCreated => "user_created",
      Self::SetupCompleted => "setup_completed",
    }
  }
}

/// Domain errors returned by the auth service.
///
/// The Tauri command layer maps each variant to a generic, user-safe
/// localized string for the frontend; the details stay in the audit log.
#[derive(Debug, Error)]
pub enum AuthError {
  #[error("invalid credentials")]
  InvalidCredentials,
  #[error("account is temporarily locked")]
  AccountLocked,
  #[error("account is inactive")]
  AccountInactive,
  #[error("initial setup is no longer available")]
  SetupUnavailable,
  #[error("username already exists")]
  UsernameTaken,
  #[error("{0}")]
  Validation(String),
  #[error("database error: {0}")]
  Database(String),
}
