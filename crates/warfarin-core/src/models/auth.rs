//! Domain models for the local-auth subsystem.
//!
//! Only types that need to cross the Tauri IPC boundary or sit on a domain
//! boundary (service ↔ repository) live here. Internal `User` rows with the
//! `password_hash` are kept out of the public DTOs.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// User role, used to gate role-based actions.
///
/// Persisted as a `&'static str` via [`UserRole::as_str`] / `FromStr`.
/// `Admin` administers the system, `Pharmacist` and `Clinician` record
/// clinical care, and `Viewer` is read-only. [`UserRole::permissions`] is
/// the single source of truth for what each role may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum UserRole {
  Admin,
  Pharmacist,
  Clinician,
  Viewer,
}

impl UserRole {
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Admin => "Admin",
      Self::Pharmacist => "Pharmacist",
      Self::Clinician => "Clinician",
      Self::Viewer => "Viewer",
    }
  }

  /// Returns every permission this role holds.
  #[must_use]
  pub const fn permissions(self) -> &'static [Permission] {
    match self {
      Self::Admin => &Permission::ALL,
      Self::Pharmacist => &[
        Permission::WriteVisit,
        Permission::ApproveVisit,
        Permission::WriteOutcome,
        Permission::WriteAppointment,
        Permission::WritePatientStatus,
        Permission::EnrollPatient,
        Permission::ManageInteractions,
      ],
      Self::Clinician => &[
        Permission::WriteVisit,
        Permission::ApproveVisit,
        Permission::WriteOutcome,
        Permission::WriteAppointment,
        Permission::WritePatientStatus,
        Permission::EnrollPatient,
      ],
      Self::Viewer => &[],
    }
  }

  /// Returns `true` when this role holds `permission`.
  #[must_use]
  pub fn allows(self, permission: Permission) -> bool {
    self.permissions().contains(&permission)
  }
}

impl std::str::FromStr for UserRole {
  type Err = String;
  fn from_str(value: &str) -> Result<Self, Self::Err> {
    match value {
      "Admin" => Ok(Self::Admin),
      "Pharmacist" => Ok(Self::Pharmacist),
      // Legacy "User" was written by migration 0011. Migration 0014 rewrites
      // the rows, but the alias keeps a not-yet-migrated database readable.
      "Clinician" | "User" => Ok(Self::Clinician),
      "Viewer" => Ok(Self::Viewer),
      other => Err(format!("unknown role: {other}")),
    }
  }
}

/// A single capability checked at the command boundary.
///
/// Commands call `require_permission`; the frontend mirrors
/// [`UserRole::permissions`] through [`PublicUser::permissions`]. The matrix
/// itself lives only in [`UserRole::permissions`], so backend enforcement
/// and UI gating cannot drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
  WriteVisit,
  ApproveVisit,
  WriteOutcome,
  WriteAppointment,
  WritePatientStatus,
  EnrollPatient,
  ManageInteractions,
  ManageSettings,
  ManageUsers,
}

impl Permission {
  /// Every permission known to the system, in stable order.
  pub const ALL: [Self; 9] = [
    Self::WriteVisit,
    Self::ApproveVisit,
    Self::WriteOutcome,
    Self::WriteAppointment,
    Self::WritePatientStatus,
    Self::EnrollPatient,
    Self::ManageInteractions,
    Self::ManageSettings,
    Self::ManageUsers,
  ];

  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::WriteVisit => "write_visit",
      Self::ApproveVisit => "approve_visit",
      Self::WriteOutcome => "write_outcome",
      Self::WriteAppointment => "write_appointment",
      Self::WritePatientStatus => "write_patient_status",
      Self::EnrollPatient => "enroll_patient",
      Self::ManageInteractions => "manage_interactions",
      Self::ManageSettings => "manage_settings",
      Self::ManageUsers => "manage_users",
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

/// Public user DTO - safe to send to the frontend over Tauri IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
  pub id: i64,
  pub username: String,
  pub role: UserRole,
  /// Capabilities granted by `role`; the frontend uses this for UI gating.
  pub permissions: Vec<Permission>,
  pub created_at: String,
}

impl From<&User> for PublicUser {
  fn from(u: &User) -> Self {
    Self {
      id: u.id,
      username: u.username.clone(),
      role: u.role,
      permissions: u.role.permissions().to_vec(),
      created_at: u.created_at.clone(),
    }
  }
}

/// In-memory session record. Lives in `AppState`; the durable copy is a
/// hashed token row in `auth_sessions`.
#[derive(Debug, Clone)]
pub struct AuthSession {
  pub user_id: i64,
  pub username: String,
  pub role: UserRole,
  pub started_at: String,
  /// Last command that used the session; refreshed on access.
  pub last_seen_at: DateTime<Utc>,
  /// Hard stop regardless of activity.
  pub absolute_expires_at: DateTime<Utc>,
  /// Idle timeout in minutes; a longer gap ends the session.
  pub idle_timeout_min: u32,
  /// SHA-256 of the persisted token, used to touch or revoke the row.
  pub token_hash: Option<String>,
}

impl AuthSession {
  #[must_use]
  pub fn public_user(&self) -> PublicUser {
    PublicUser {
      id: self.user_id,
      username: self.username.clone(),
      role: self.role,
      permissions: self.role.permissions().to_vec(),
      created_at: self.started_at.clone(),
    }
  }

  /// Returns `true` when the idle or absolute limit has passed.
  #[must_use]
  pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
    now >= self.absolute_expires_at
      || now.signed_duration_since(self.last_seen_at)
        >= Duration::minutes(i64::from(self.idle_timeout_min))
  }

  /// Moves the idle window forward.
  pub fn touch(&mut self, now: DateTime<Utc>) {
    self.last_seen_at = now;
  }
}

/// Result of a successful `login` / `setup_admin`: the public user plus the
/// raw session token the command layer stores in the OS keychain.
#[derive(Debug, Clone)]
pub struct AuthenticatedSession {
  pub user: PublicUser,
  pub token: String,
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

/// Admin-created account (frontend → `create_user` command).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserInput {
  pub username: String,
  pub password: String,
  pub role: UserRole,
}

/// Admin list row for the user management screen.
///
/// Unlike [`PublicUser`] this carries account state (active flag, lockout)
/// that an administrator needs to see. It still never contains the hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedUser {
  pub id: i64,
  pub username: String,
  pub role: UserRole,
  pub active: bool,
  pub locked_until: Option<String>,
  pub created_at: String,
}

impl From<&User> for ManagedUser {
  fn from(u: &User) -> Self {
    Self {
      id: u.id,
      username: u.username.clone(),
      role: u.role,
      active: u.active,
      locked_until: u.locked_until.clone(),
      created_at: u.created_at.clone(),
    }
  }
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
  PasswordReset,
  RoleChanged,
  UserActivated,
  UserDeactivated,
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
      Self::PasswordReset => "password_reset",
      Self::RoleChanged => "role_changed",
      Self::UserActivated => "user_activated",
      Self::UserDeactivated => "user_deactivated",
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
  #[error("user not found")]
  UserNotFound,
  #[error("the last active administrator cannot be demoted or disabled")]
  LastAdmin,
  #[error("an administrator cannot modify their own account here")]
  SelfModification,
  #[error("{0}")]
  Validation(String),
  #[error("database error: {0}")]
  Database(String),
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn admin_holds_every_permission() {
    for permission in Permission::ALL {
      assert!(
        UserRole::Admin.allows(permission),
        "admin is missing {permission:?}"
      );
    }
  }

  #[test]
  fn viewer_is_read_only() {
    assert!(UserRole::Viewer.permissions().is_empty());
    for permission in Permission::ALL {
      assert!(!UserRole::Viewer.allows(permission));
    }
  }

  #[test]
  fn clinician_records_care_but_does_not_administer() {
    assert!(UserRole::Clinician.allows(Permission::WriteVisit));
    assert!(UserRole::Clinician.allows(Permission::ApproveVisit));
    assert!(UserRole::Clinician.allows(Permission::WriteOutcome));
    assert!(!UserRole::Clinician.allows(Permission::ManageInteractions));
    assert!(!UserRole::Clinician.allows(Permission::ManageSettings));
    assert!(!UserRole::Clinician.allows(Permission::ManageUsers));
  }

  #[test]
  fn pharmacist_manages_interactions_but_not_settings() {
    assert!(UserRole::Pharmacist.allows(Permission::WriteVisit));
    assert!(UserRole::Pharmacist.allows(Permission::ManageInteractions));
    assert!(!UserRole::Pharmacist.allows(Permission::ManageSettings));
    assert!(!UserRole::Pharmacist.allows(Permission::ManageUsers));
  }

  #[test]
  fn role_strings_round_trip_and_legacy_user_maps_to_clinician() {
    for role in [
      UserRole::Admin,
      UserRole::Pharmacist,
      UserRole::Clinician,
      UserRole::Viewer,
    ] {
      assert_eq!(role.as_str().parse::<UserRole>(), Ok(role));
    }
    assert_eq!("User".parse::<UserRole>(), Ok(UserRole::Clinician));
    assert!("Wizard".parse::<UserRole>().is_err());
  }

  #[test]
  fn permission_serde_uses_snake_case_strings() {
    let json = serde_json::to_string(&Permission::WriteVisit).expect("serialize permission");
    assert_eq!(json, "\"write_visit\"");
  }

  #[test]
  fn public_user_carries_the_role_permissions() {
    let user = User {
      id: 1,
      username: "somchai".to_string(),
      password_hash: "hash".to_string(),
      role: UserRole::Pharmacist,
      active: true,
      failed_attempts: 0,
      locked_until: None,
      created_at: "2026-01-01T00:00:00Z".to_string(),
      updated_at: "2026-01-01T00:00:00Z".to_string(),
    };
    let public = PublicUser::from(&user);
    assert_eq!(
      public.permissions,
      UserRole::Pharmacist.permissions().to_vec()
    );
  }

  #[test]
  fn managed_user_exposes_account_state_without_the_hash() {
    let user = User {
      id: 7,
      username: "somsri".to_string(),
      password_hash: "top-secret-hash".to_string(),
      role: UserRole::Viewer,
      active: false,
      failed_attempts: 3,
      locked_until: Some("2026-01-01T00:15:00Z".to_string()),
      created_at: "2026-01-01T00:00:00Z".to_string(),
      updated_at: "2026-01-01T00:00:00Z".to_string(),
    };
    let managed = ManagedUser::from(&user);
    assert_eq!(managed.id, 7);
    assert_eq!(managed.role, UserRole::Viewer);
    assert!(!managed.active);
    assert_eq!(
      managed.locked_until.as_deref(),
      Some("2026-01-01T00:15:00Z")
    );
  }

  #[test]
  fn audit_event_strings_are_unique() {
    let mut seen = Vec::new();
    for event in [
      AuthEventType::LoginSuccess,
      AuthEventType::LoginFailed,
      AuthEventType::AccountLocked,
      AuthEventType::Logout,
      AuthEventType::UserCreated,
      AuthEventType::SetupCompleted,
      AuthEventType::PasswordReset,
      AuthEventType::RoleChanged,
      AuthEventType::UserActivated,
      AuthEventType::UserDeactivated,
    ] {
      assert!(
        !seen.contains(&event.as_str()),
        "duplicate {}",
        event.as_str()
      );
      seen.push(event.as_str());
    }
  }

  fn session_at(now: DateTime<Utc>) -> AuthSession {
    AuthSession {
      user_id: 1,
      username: "admin1".to_string(),
      role: UserRole::Admin,
      started_at: now.to_rfc3339(),
      last_seen_at: now,
      absolute_expires_at: now + Duration::hours(8),
      idle_timeout_min: 30,
      token_hash: Some("hash".to_string()),
    }
  }

  #[test]
  fn session_expires_after_the_idle_window() {
    let now = Utc::now();
    let session = session_at(now);
    assert!(!session.is_expired(now + Duration::minutes(29)));
    assert!(session.is_expired(now + Duration::minutes(30)));
  }

  #[test]
  fn session_expires_at_the_absolute_limit_even_when_active() {
    let now = Utc::now();
    let session = session_at(now);
    assert!(session.is_expired(now + Duration::hours(8)));
  }

  #[test]
  fn touching_a_session_moves_the_idle_window() {
    let now = Utc::now();
    let mut session = session_at(now);
    let later = now + Duration::minutes(20);
    session.touch(later);
    assert!(!session.is_expired(later + Duration::minutes(29)));
    assert!(session.is_expired(later + Duration::minutes(30)));
  }
}
