//! Authentication service.
//!
//! Owns the auth business rules: credential validation, rate limiting, and
//! the in-memory session slot. Persists audit events but never logs
//! passwords. Calls [`warfarin_core::auth`] for pure crypto and validation
//! and [`auth_repository`] for `SQLite` access.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use warfarin_core::auth::{
  MAX_FAILED_ATTEMPTS, hash_password, lockout_until_now, now_rfc3339, validate_password_strength,
  validate_username, verify_password,
};
use warfarin_core::models::auth::{
  AuthError, AuthEventType, AuthSession, CreateUserInput, LoginInput, ManagedUser, PublicUser,
  SetupAdminInput, UserRole,
};

use crate::auth_repository;

/// Convenience alias for the in-memory session slot kept in `AppState`.
pub type AuthSessionSlot = Arc<Mutex<Option<AuthSession>>>;

/// Returns `true` if at least one user has been created (used to decide
/// whether to show the first-time setup screen).
pub async fn has_users(pool: &SqlitePool) -> bool {
  match auth_repository::user_count(pool).await {
    Ok(n) => n > 0,
    Err(e) => {
      eprintln!("[auth] user_count failed: {e:#}");
      false
    }
  }
}

/// Creates the first administrator account. Refuses to run when any user
/// already exists.
///
/// On success, populates `session` with the new admin's [`AuthSession`].
pub async fn setup_admin(
  pool: &SqlitePool,
  session: &AuthSessionSlot,
  input: SetupAdminInput,
) -> Result<PublicUser, AuthError> {
  if has_users(pool).await {
    return Err(AuthError::SetupUnavailable);
  }

  let username = input.username.trim().to_string();
  validate_username(&username).map_err(AuthError::Validation)?;
  validate_password_strength(&input.password).map_err(AuthError::Validation)?;

  let hash = hash_password(&input.password).map_err(AuthError::Validation)?;

  let new_id = auth_repository::insert_user(pool, &username, &hash, UserRole::Admin)
    .await
    .map_err(|e| map_repo_err(&e))?;

  let _ = auth_repository::insert_audit(
    pool,
    AuthEventType::UserCreated,
    &username,
    true,
    Some("first-time setup"),
  )
  .await;
  let _ =
    auth_repository::insert_audit(pool, AuthEventType::SetupCompleted, &username, true, None).await;

  let started_at = now_rfc3339();
  let sess = AuthSession {
    user_id: new_id,
    username: username.clone(),
    role: UserRole::Admin,
    started_at: started_at.clone(),
  };
  *session.lock().await = Some(sess);

  Ok(PublicUser {
    id: new_id,
    username,
    role: UserRole::Admin,
    permissions: UserRole::Admin.permissions().to_vec(),
    created_at: started_at,
  })
}

/// Authenticates a user.
///
/// Failure cases that come from "no such user" and "wrong password" both
/// resolve to [`AuthError::InvalidCredentials`] and share the same dummy-hash
/// timing path so a network observer cannot enumerate usernames.
///
/// # Panics
///
/// Panics if the first-time dummy Argon2id hash generation fails. The
/// underlying `Argon2::default().hash_password` call only fails when given
/// an empty password, so the panic cannot fire for the hard-coded input
/// passed in.
pub async fn login(
  pool: &SqlitePool,
  session: &AuthSessionSlot,
  input: LoginInput,
) -> Result<PublicUser, AuthError> {
  let username = input.username.trim().to_string();
  if username.is_empty() {
    return Err(AuthError::InvalidCredentials);
  }

  // Always run a verify call so the timing of "user not found" matches
  // "user found but wrong password". The dummy hash is generated once per
  // process and is intentionally a valid Argon2id PHC string.
  let dummy_hash = DUMMY_HASH.get_or_init(|| {
    hash_password("dummy-do-not-use").expect("dummy hash generation should not fail")
  });

  let user = match auth_repository::find_by_username(pool, &username).await {
    Ok(Some(u)) => u,
    Ok(None) => {
      // Equalize timing.
      let _ = verify_password(&input.password, dummy_hash);
      let _ = auth_repository::insert_audit(
        pool,
        AuthEventType::LoginFailed,
        &username,
        false,
        Some("unknown user"),
      )
      .await;
      return Err(AuthError::InvalidCredentials);
    }
    Err(e) => return Err(AuthError::Database(format!("{e:#}"))),
  };

  if let Some(locked_until) = user.locked_until.as_deref() {
    if is_locked(locked_until) {
      let _ = auth_repository::insert_audit(
        pool,
        AuthEventType::AccountLocked,
        &username,
        false,
        Some("attempted login while locked"),
      )
      .await;
      return Err(AuthError::AccountLocked);
    }
  }

  if !user.active {
    let _ = auth_repository::insert_audit(
      pool,
      AuthEventType::LoginFailed,
      &username,
      false,
      Some("inactive account"),
    )
    .await;
    return Err(AuthError::AccountInactive);
  }

  let password_ok = verify_password(&input.password, &user.password_hash)
    .map_err(|e| AuthError::Database(format!("verify_password: {e}")))?;

  if !password_ok {
    let new_count = auth_repository::record_failed_attempt(pool, user.id)
      .await
      .map_err(|e| map_repo_err(&e))?;
    let now_locked = new_count >= MAX_FAILED_ATTEMPTS;
    if now_locked {
      let until = lockout_until_now();
      let _ = auth_repository::set_locked_until(pool, user.id, &until).await;
    }
    let _ = auth_repository::insert_audit(
      pool,
      AuthEventType::LoginFailed,
      &username,
      false,
      Some(if now_locked {
        "locked"
      } else {
        "wrong password"
      }),
    )
    .await;
    return Err(AuthError::InvalidCredentials);
  }

  auth_repository::reset_failed_attempts(pool, user.id)
    .await
    .map_err(|e| map_repo_err(&e))?;
  let _ =
    auth_repository::insert_audit(pool, AuthEventType::LoginSuccess, &username, true, None).await;

  let started_at = now_rfc3339();
  let public = PublicUser {
    id: user.id,
    username: username.clone(),
    role: user.role,
    permissions: user.role.permissions().to_vec(),
    created_at: started_at.clone(),
  };
  *session.lock().await = Some(AuthSession {
    user_id: user.id,
    username,
    role: user.role,
    started_at,
  });
  Ok(public)
}

/// Clears the in-memory session. Always succeeds; safe to call repeatedly.
pub async fn logout(pool: &SqlitePool, session: &AuthSessionSlot) {
  let username_opt = {
    let guard = session.lock().await;
    guard.as_ref().map(|s| s.username.clone())
  };
  *session.lock().await = None;
  if let Some(username) = username_opt {
    let _ = auth_repository::insert_audit(pool, AuthEventType::Logout, &username, true, None).await;
  }
}

/// Returns the public user view for the current session, if any.
pub async fn current_user(session: &AuthSessionSlot) -> Option<PublicUser> {
  session.lock().await.as_ref().map(AuthSession::public_user)
}

/// Returns `true` when an in-memory session is present.
pub async fn is_logged_in(session: &AuthSessionSlot) -> bool {
  session.lock().await.is_some()
}

/// Lists every account for the admin user management screen.
pub async fn list_users(pool: &SqlitePool) -> Result<Vec<ManagedUser>, AuthError> {
  auth_repository::list_users(pool)
    .await
    .map(|users| users.iter().map(ManagedUser::from).collect())
    .map_err(|e| map_repo_err(&e))
}

/// Creates a staff account with an explicit role.
///
/// `actor` is the logged-in administrator; their name is recorded in the
/// audit log, never the new account's password.
pub async fn create_user(
  pool: &SqlitePool,
  actor: &PublicUser,
  input: CreateUserInput,
) -> Result<ManagedUser, AuthError> {
  let username = input.username.trim().to_string();
  validate_username(&username).map_err(AuthError::Validation)?;
  validate_password_strength(&input.password).map_err(AuthError::Validation)?;

  let hash = hash_password(&input.password).map_err(AuthError::Validation)?;
  let new_id = auth_repository::insert_user(pool, &username, &hash, input.role)
    .await
    .map_err(|e| map_repo_err(&e))?;

  let _ = auth_repository::insert_audit(
    pool,
    AuthEventType::UserCreated,
    &username,
    true,
    Some(&format!(
      "created by {} with role {}",
      actor.username,
      input.role.as_str()
    )),
  )
  .await;

  let user = auth_repository::find_by_id(pool, new_id)
    .await
    .map_err(|e| map_repo_err(&e))?
    .ok_or(AuthError::UserNotFound)?;
  Ok(ManagedUser::from(&user))
}

/// Replaces a user's password. Also clears their lockout state.
pub async fn reset_password(
  pool: &SqlitePool,
  actor: &PublicUser,
  user_id: i64,
  new_password: &str,
) -> Result<(), AuthError> {
  validate_password_strength(new_password).map_err(AuthError::Validation)?;
  let target = auth_repository::find_by_id(pool, user_id)
    .await
    .map_err(|e| map_repo_err(&e))?
    .ok_or(AuthError::UserNotFound)?;

  let hash = hash_password(new_password).map_err(AuthError::Validation)?;
  auth_repository::update_password(pool, user_id, &hash)
    .await
    .map_err(|e| map_repo_err(&e))?;

  let _ = auth_repository::insert_audit(
    pool,
    AuthEventType::PasswordReset,
    &target.username,
    true,
    Some(&format!("reset by {}", actor.username)),
  )
  .await;
  Ok(())
}

/// Changes another user's role. Refuses to touch the actor's own account and
/// refuses to demote the last enabled administrator.
pub async fn set_role(
  pool: &SqlitePool,
  actor: &PublicUser,
  user_id: i64,
  role: UserRole,
) -> Result<(), AuthError> {
  if user_id == actor.id {
    return Err(AuthError::SelfModification);
  }
  let target = auth_repository::find_by_id(pool, user_id)
    .await
    .map_err(|e| map_repo_err(&e))?
    .ok_or(AuthError::UserNotFound)?;

  let loses_admin = target.role == UserRole::Admin && role != UserRole::Admin;
  if loses_admin && auth_repository::count_active_admins(pool).await.map_err(|e| map_repo_err(&e))? <= 1
  {
    return Err(AuthError::LastAdmin);
  }

  auth_repository::set_role(pool, user_id, role)
    .await
    .map_err(|e| map_repo_err(&e))?;

  let _ = auth_repository::insert_audit(
    pool,
    AuthEventType::RoleChanged,
    &target.username,
    true,
    Some(&format!(
      "{} -> {} by {}",
      target.role.as_str(),
      role.as_str(),
      actor.username
    )),
  )
  .await;
  Ok(())
}

/// Enables or disables an account. Refuses to disable the actor's own account
/// and refuses to disable the last enabled administrator.
pub async fn set_active(
  pool: &SqlitePool,
  actor: &PublicUser,
  user_id: i64,
  active: bool,
) -> Result<(), AuthError> {
  if user_id == actor.id && !active {
    return Err(AuthError::SelfModification);
  }
  let target = auth_repository::find_by_id(pool, user_id)
    .await
    .map_err(|e| map_repo_err(&e))?
    .ok_or(AuthError::UserNotFound)?;

  let loses_admin = target.active && !active && target.role == UserRole::Admin;
  if loses_admin && auth_repository::count_active_admins(pool).await.map_err(|e| map_repo_err(&e))? <= 1
  {
    return Err(AuthError::LastAdmin);
  }

  auth_repository::set_active(pool, user_id, active)
    .await
    .map_err(|e| map_repo_err(&e))?;

  let event = if active {
    AuthEventType::UserActivated
  } else {
    AuthEventType::UserDeactivated
  };
  let _ = auth_repository::insert_audit(
    pool,
    event,
    &target.username,
    true,
    Some(&format!("by {}", actor.username)),
  )
  .await;
  Ok(())
}

fn is_locked(locked_until: &str) -> bool {
  match DateTime::parse_from_rfc3339(locked_until) {
    Ok(ts) => ts.with_timezone(&Utc) > Utc::now(),
    Err(_) => false,
  }
}

fn map_repo_err(e: &anyhow::Error) -> AuthError {
  let msg = format!("{e:#}");
  let lower = msg.to_ascii_lowercase();
  // sqlx reports UNIQUE constraint violations as `...UNIQUE constraint failed...`.
  if lower.contains("unique constraint failed") {
    AuthError::UsernameTaken
  } else {
    AuthError::Database(msg)
  }
}

// A process-wide cache for the timing-equalization dummy hash. `OnceLock`
// is sync; the actual hash value is generated by `hash_password` which
// itself spawns no tasks, so a blocking init here is fine.
use std::sync::OnceLock;
static DUMMY_HASH: OnceLock<String> = OnceLock::new();

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::sqlite::SqlitePoolOptions;

  async fn in_memory_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
      .max_connections(1)
      .connect("sqlite::memory:")
      .await
      .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
  }

  #[tokio::test]
  async fn setup_then_login_round_trip() {
    let pool = in_memory_pool().await;
    let session: AuthSessionSlot = Arc::new(Mutex::new(None));

    assert!(!has_users(&pool).await);

    let admin = setup_admin(
      &pool,
      &session,
      SetupAdminInput {
        username: "admin".to_string(),
        password: "Password1".to_string(),
      },
    )
    .await
    .unwrap();
    assert_eq!(admin.username, "admin");

    // Second setup attempt must fail.
    let again = setup_admin(
      &pool,
      &session,
      SetupAdminInput {
        username: "admin2".to_string(),
        password: "Password2".to_string(),
      },
    )
    .await;
    assert!(matches!(again, Err(AuthError::SetupUnavailable)));

    // Logout, then login.
    logout(&pool, &session).await;
    assert!(!is_logged_in(&session).await);

    let logged = login(
      &pool,
      &session,
      LoginInput {
        username: "admin".to_string(),
        password: "Password1".to_string(),
      },
    )
    .await
    .unwrap();
    assert_eq!(logged.username, "admin");
    assert!(is_logged_in(&session).await);
  }

  #[tokio::test]
  async fn wrong_password_increments_counter_and_locks() {
    let pool = in_memory_pool().await;
    let session: AuthSessionSlot = Arc::new(Mutex::new(None));

    let _ = setup_admin(
      &pool,
      &session,
      SetupAdminInput {
        username: "admin".to_string(),
        password: "Password1".to_string(),
      },
    )
    .await
    .unwrap();
    logout(&pool, &session).await;

    for i in 0..MAX_FAILED_ATTEMPTS {
      let r = login(
        &pool,
        &session,
        LoginInput {
          username: "admin".to_string(),
          password: "wrong".to_string(),
        },
      )
      .await;
      assert!(matches!(r, Err(AuthError::InvalidCredentials)), "iter {i}");
    }
    // Next attempt must be locked even with the right password.
    let r = login(
      &pool,
      &session,
      LoginInput {
        username: "admin".to_string(),
        password: "Password1".to_string(),
      },
    )
    .await;
    assert!(matches!(r, Err(AuthError::AccountLocked)));
  }

  #[tokio::test]
  async fn login_unknown_user_returns_invalid_credentials() {
    let pool = in_memory_pool().await;
    let session: AuthSessionSlot = Arc::new(Mutex::new(None));
    let r = login(
      &pool,
      &session,
      LoginInput {
        username: "ghost".to_string(),
        password: "Password1".to_string(),
      },
    )
    .await;
    assert!(matches!(r, Err(AuthError::InvalidCredentials)));
  }

  #[tokio::test]
  async fn setup_rejects_weak_password() {
    let pool = in_memory_pool().await;
    let session: AuthSessionSlot = Arc::new(Mutex::new(None));
    let r = setup_admin(
      &pool,
      &session,
      SetupAdminInput {
        username: "admin".to_string(),
        password: "short".to_string(),
      },
    )
    .await;
    assert!(matches!(r, Err(AuthError::Validation(_))));
  }

  fn actor(id: i64, username: &str) -> PublicUser {
    PublicUser {
      id,
      username: username.to_string(),
      role: UserRole::Admin,
      permissions: UserRole::Admin.permissions().to_vec(),
      created_at: "2026-01-01T00:00:00Z".to_string(),
    }
  }

  async fn seed_user(pool: &SqlitePool, username: &str, role: UserRole) -> i64 {
    let hash = hash_password("Password1").unwrap();
    auth_repository::insert_user(pool, username, &hash, role)
      .await
      .unwrap()
  }

  #[tokio::test]
  async fn create_user_lists_the_account_and_writes_audit() {
    let pool = in_memory_pool().await;
    let admin = actor(1, "admin1");
    seed_user(&pool, "admin1", UserRole::Admin).await;

    let created = create_user(
      &pool,
      &admin,
      CreateUserInput {
        username: "pharm1".to_string(),
        password: "Password1".to_string(),
        role: UserRole::Pharmacist,
      },
    )
    .await
    .unwrap();

    assert_eq!(created.username, "pharm1");
    assert_eq!(created.role, UserRole::Pharmacist);
    assert!(created.active);

    let users = list_users(&pool).await.unwrap();
    assert_eq!(users.len(), 2);
    assert!(users.iter().any(|u| u.username == "pharm1"));

    let audit_count: i64 = sqlx::query_scalar(
      "SELECT COUNT(*) FROM auth_audit_log \
        WHERE event_type = 'user_created' AND username = 'pharm1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 1);
  }

  #[tokio::test]
  async fn reset_password_lets_the_user_log_in_with_the_new_password() {
    let pool = in_memory_pool().await;
    let admin = actor(1, "admin1");
    seed_user(&pool, "admin1", UserRole::Admin).await;
    let created = create_user(
      &pool,
      &admin,
      CreateUserInput {
        username: "pharm1".to_string(),
        password: "Password1".to_string(),
        role: UserRole::Pharmacist,
      },
    )
    .await
    .unwrap();

    reset_password(&pool, &admin, created.id, "NewPassword2")
      .await
      .unwrap();

    let session: AuthSessionSlot = Arc::new(Mutex::new(None));
    let ok = login(
      &pool,
      &session,
      LoginInput {
        username: "pharm1".to_string(),
        password: "NewPassword2".to_string(),
      },
    )
    .await;
    assert!(ok.is_ok());

    logout(&pool, &session).await;
    let old = login(
      &pool,
      &session,
      LoginInput {
        username: "pharm1".to_string(),
        password: "Password1".to_string(),
      },
    )
    .await;
    assert!(matches!(old, Err(AuthError::InvalidCredentials)));
  }

  #[tokio::test]
  async fn disabled_user_cannot_log_in() {
    let pool = in_memory_pool().await;
    let admin = actor(1, "admin1");
    seed_user(&pool, "admin1", UserRole::Admin).await;
    let created = create_user(
      &pool,
      &admin,
      CreateUserInput {
        username: "clin1".to_string(),
        password: "Password1".to_string(),
        role: UserRole::Clinician,
      },
    )
    .await
    .unwrap();

    set_active(&pool, &admin, created.id, false).await.unwrap();

    let session: AuthSessionSlot = Arc::new(Mutex::new(None));
    let r = login(
      &pool,
      &session,
      LoginInput {
        username: "clin1".to_string(),
        password: "Password1".to_string(),
      },
    )
    .await;
    assert!(matches!(r, Err(AuthError::AccountInactive)));
  }

  #[tokio::test]
  async fn last_active_admin_cannot_be_demoted_or_disabled() {
    let pool = in_memory_pool().await;
    // The actor is an admin id that does not exist in the table, so the
    // only real account is the seeded active admin: the last one.
    let admin = actor(99, "other-admin");
    let only_admin_id = seed_user(&pool, "admin1", UserRole::Admin).await;

    let demote = set_role(&pool, &admin, only_admin_id, UserRole::Viewer).await;
    assert!(matches!(demote, Err(AuthError::LastAdmin)));

    let disable = set_active(&pool, &admin, only_admin_id, false).await;
    assert!(matches!(disable, Err(AuthError::LastAdmin)));

    // Adding a second active admin unlocks both operations.
    let second = seed_user(&pool, "admin2", UserRole::Admin).await;
    assert!(set_role(&pool, &admin, second, UserRole::Viewer).await.is_ok());
  }

  #[tokio::test]
  async fn self_role_change_and_self_disable_are_rejected() {
    let pool = in_memory_pool().await;
    let admin = actor(1, "admin1");
    seed_user(&pool, "admin1", UserRole::Admin).await;

    let demote_self = set_role(&pool, &admin, admin.id, UserRole::Viewer).await;
    assert!(matches!(demote_self, Err(AuthError::SelfModification)));

    let disable_self = set_active(&pool, &admin, admin.id, false).await;
    assert!(matches!(disable_self, Err(AuthError::SelfModification)));

    // Re-enabling self is harmless and stays allowed.
    assert!(set_active(&pool, &admin, admin.id, true).await.is_ok());
  }
}
