//! Auth-related `SQLite` access. One file per feature so the rest of the data
//! layer stays focused on clinic records.

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use std::str::FromStr;

use warfarin_core::auth::{now_rfc3339, validate_username};
use warfarin_core::models::auth::{AuthEventType, User, UserRole};

fn parse_user_row(row: &sqlx::sqlite::SqliteRow) -> Result<User> {
  let role_raw: String = row.get("role");
  let role = UserRole::from_str(&role_raw)
    .map_err(|e| anyhow::anyhow!("invalid role in users table: {e}"))?;
  Ok(User {
    id: row.get("id"),
    username: row.get("username"),
    password_hash: row.get("password_hash"),
    role,
    active: {
      let n: i64 = row.get("active");
      n != 0
    },
    failed_attempts: {
      let n: i64 = row.get("failed_attempts");
      u32::try_from(n).with_context(|| format!("failed_attempts out of range: {n}"))?
    },
    locked_until: row.get("locked_until"),
    created_at: row.get("created_at"),
    updated_at: row.get("updated_at"),
  })
}

/// Returns the total number of users in the `users` table.
pub async fn user_count(pool: &SqlitePool) -> Result<i64> {
  let row = sqlx::query("SELECT COUNT(*) AS cnt FROM users")
    .fetch_one(pool)
    .await
    .context("failed to count users")?;
  Ok(row.get("cnt"))
}

/// Looks up a user by username (case-insensitive via `COLLATE NOCASE` on the
/// column's `UNIQUE` index is not enforced - we trim/normalize at the service
/// layer and rely on the `UNIQUE` constraint to catch duplicates on insert).
pub async fn find_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>> {
  let row = sqlx::query(
    "SELECT id, username, password_hash, role, active, failed_attempts, \
            locked_until, created_at, updated_at \
       FROM users WHERE username = ?",
  )
  .bind(username)
  .fetch_optional(pool)
  .await
  .context("failed to query user")?;
  row.map(|r| parse_user_row(&r)).transpose()
}

/// Looks up a user by primary key.
pub async fn find_by_id(pool: &SqlitePool, user_id: i64) -> Result<Option<User>> {
  let row = sqlx::query(
    "SELECT id, username, password_hash, role, active, failed_attempts, \
            locked_until, created_at, updated_at \
       FROM users WHERE id = ?",
  )
  .bind(user_id)
  .fetch_optional(pool)
  .await
  .context("failed to query user by id")?;
  row.map(|r| parse_user_row(&r)).transpose()
}

/// Lists every user ordered by username (case-insensitive).
pub async fn list_users(pool: &SqlitePool) -> Result<Vec<User>> {
  let rows = sqlx::query(
    "SELECT id, username, password_hash, role, active, failed_attempts, \
            locked_until, created_at, updated_at \
       FROM users ORDER BY username COLLATE NOCASE",
  )
  .fetch_all(pool)
  .await
  .context("failed to list users")?;
  rows.iter().map(parse_user_row).collect()
}

/// Updates a user's role.
pub async fn set_role(pool: &SqlitePool, user_id: i64, role: UserRole) -> Result<()> {
  let now = now_rfc3339();
  let updated = sqlx::query("UPDATE users SET role = ?, updated_at = ? WHERE id = ?")
    .bind(role.as_str())
    .bind(&now)
    .bind(user_id)
    .execute(pool)
    .await
    .context("failed to update user role")?;
  if updated.rows_affected() == 0 {
    anyhow::bail!("user not found: {user_id}");
  }
  Ok(())
}

/// Enables or disables an account. Enabling clears the lockout state so the
/// user can log in immediately; disabling leaves lockout bookkeeping alone.
pub async fn set_active(pool: &SqlitePool, user_id: i64, active: bool) -> Result<()> {
  let now = now_rfc3339();
  let flag = i64::from(active);
  let updated = sqlx::query(
    "UPDATE users \
        SET active = ?, \
            failed_attempts = CASE WHEN ? THEN 0 ELSE failed_attempts END, \
            locked_until = CASE WHEN ? THEN NULL ELSE locked_until END, \
            updated_at = ? \
      WHERE id = ?",
  )
  .bind(flag)
  .bind(flag)
  .bind(flag)
  .bind(&now)
  .bind(user_id)
  .execute(pool)
  .await
  .context("failed to update user active flag")?;
  if updated.rows_affected() == 0 {
    anyhow::bail!("user not found: {user_id}");
  }
  Ok(())
}

/// Replaces a user's password hash and clears lockout state.
pub async fn update_password(pool: &SqlitePool, user_id: i64, password_hash: &str) -> Result<()> {
  let now = now_rfc3339();
  let updated = sqlx::query(
    "UPDATE users \
        SET password_hash = ?, failed_attempts = 0, locked_until = NULL, updated_at = ? \
      WHERE id = ?",
  )
  .bind(password_hash)
  .bind(&now)
  .bind(user_id)
  .execute(pool)
  .await
  .context("failed to update password hash")?;
  if updated.rows_affected() == 0 {
    anyhow::bail!("user not found: {user_id}");
  }
  Ok(())
}

/// Counts enabled administrator accounts, used for the last-admin guard.
pub async fn count_active_admins(pool: &SqlitePool) -> Result<i64> {
  let row = sqlx::query("SELECT COUNT(*) AS cnt FROM users WHERE role = 'Admin' AND active = 1")
    .fetch_one(pool)
    .await
    .context("failed to count active admins")?;
  Ok(row.get("cnt"))
}

// auth_sessions

/// Durable session row backing a persisted login.
#[derive(Debug, Clone)]
pub struct SessionRow {
  pub id: i64,
  pub user_id: i64,
  pub created_at: String,
  pub last_seen_at: String,
  pub expires_at: String,
}

/// Inserts a session row for a freshly issued token hash.
pub async fn insert_session(
  pool: &SqlitePool,
  token_hash: &str,
  user_id: i64,
  machine_id: &str,
  created_at: &str,
  last_seen_at: &str,
  expires_at: &str,
) -> Result<()> {
  sqlx::query(
    "INSERT INTO auth_sessions \
        (token_hash, user_id, machine_id, created_at, last_seen_at, expires_at) \
        VALUES (?, ?, ?, ?, ?, ?)",
  )
  .bind(token_hash)
  .bind(user_id)
  .bind(machine_id)
  .bind(created_at)
  .bind(last_seen_at)
  .bind(expires_at)
  .execute(pool)
  .await
  .context("failed to insert session")?;
  Ok(())
}

/// Looks up a non-revoked session by token hash.
pub async fn find_active_session(
  pool: &SqlitePool,
  token_hash: &str,
) -> Result<Option<SessionRow>> {
  let row = sqlx::query(
    "SELECT id, user_id, created_at, last_seen_at, expires_at \
       FROM auth_sessions \
      WHERE token_hash = ? AND revoked_at IS NULL",
  )
  .bind(token_hash)
  .fetch_optional(pool)
  .await
  .context("failed to query session")?;
  Ok(row.map(|r| SessionRow {
    id: r.get("id"),
    user_id: r.get("user_id"),
    created_at: r.get("created_at"),
    last_seen_at: r.get("last_seen_at"),
    expires_at: r.get("expires_at"),
  }))
}

/// Refreshes `last_seen_at` for a session.
pub async fn touch_session(pool: &SqlitePool, token_hash: &str, last_seen_at: &str) -> Result<()> {
  sqlx::query(
    "UPDATE auth_sessions SET last_seen_at = ? WHERE token_hash = ? AND revoked_at IS NULL",
  )
  .bind(last_seen_at)
  .bind(token_hash)
  .execute(pool)
  .await
  .context("failed to touch session")?;
  Ok(())
}

/// Marks a session revoked. Unknown hashes are ignored.
pub async fn revoke_session(pool: &SqlitePool, token_hash: &str, revoked_at: &str) -> Result<()> {
  sqlx::query(
    "UPDATE auth_sessions SET revoked_at = ? WHERE token_hash = ? AND revoked_at IS NULL",
  )
  .bind(revoked_at)
  .bind(token_hash)
  .execute(pool)
  .await
  .context("failed to revoke session")?;
  Ok(())
}

/// Deletes revoked and expired session rows. Returns the number removed.
pub async fn prune_sessions(pool: &SqlitePool, now: &str) -> Result<u64> {
  let result =
    sqlx::query("DELETE FROM auth_sessions WHERE revoked_at IS NOT NULL OR expires_at < ?")
      .bind(now)
      .execute(pool)
      .await
      .context("failed to prune sessions")?;
  Ok(result.rows_affected())
}

/// Records the timestamp of a successful login on the user row.
pub async fn set_last_login(pool: &SqlitePool, user_id: i64, at: &str) -> Result<()> {
  sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
    .bind(at)
    .bind(user_id)
    .execute(pool)
    .await
    .context("failed to set last_login_at")?;
  Ok(())
}

/// Returns the most recent successful login across all accounts, for the
/// login screen. Deliberately machine-wide so it cannot be used to probe
/// whether a specific username exists.
pub async fn last_login_hint(pool: &SqlitePool) -> Result<Option<String>> {
  let row = sqlx::query("SELECT MAX(last_login_at) AS last_login FROM users")
    .fetch_one(pool)
    .await
    .context("failed to read last login hint")?;
  Ok(row.try_get("last_login").ok().flatten())
}

/// Inserts a new user. Returns the new row ID.
///
/// # Errors
///
/// Returns an `Err` if the username fails local validation, the row already
/// exists, or the underlying SQL fails.
pub async fn insert_user(
  pool: &SqlitePool,
  username: &str,
  password_hash: &str,
  role: UserRole,
) -> Result<i64> {
  validate_username(username).map_err(anyhow::Error::msg)?;
  let now = now_rfc3339();
  let id = sqlx::query(
    "INSERT INTO users (username, password_hash, role, active, failed_attempts, \
                         created_at, updated_at) \
        VALUES (?, ?, ?, 1, 0, ?, ?)",
  )
  .bind(username.trim())
  .bind(password_hash)
  .bind(role.as_str())
  .bind(&now)
  .bind(&now)
  .execute(pool)
  .await
  .context("failed to insert user")?
  .last_insert_rowid();
  Ok(id)
}

/// Increments `failed_attempts` and returns the new value.
pub async fn record_failed_attempt(pool: &SqlitePool, user_id: i64) -> Result<u32> {
  let now = now_rfc3339();
  let mut tx = pool.begin().await.context("failed to begin tx")?;
  let updated = sqlx::query(
    "UPDATE users SET failed_attempts = failed_attempts + 1, updated_at = ? \
        WHERE id = ?",
  )
  .bind(&now)
  .bind(user_id)
  .execute(&mut *tx)
  .await
  .context("failed to increment failed_attempts")?;
  if updated.rows_affected() == 0 {
    anyhow::bail!("user not found: {user_id}");
  }
  let new_count: i64 = sqlx::query("SELECT failed_attempts AS cnt FROM users WHERE id = ?")
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .context("failed to read failed_attempts")?
    .get("cnt");
  tx.commit().await.context("failed to commit")?;
  u32::try_from(new_count).with_context(|| format!("failed_attempts out of range: {new_count}"))
}

/// Sets `locked_until` to the given RFC 3339 timestamp.
pub async fn set_locked_until(pool: &SqlitePool, user_id: i64, locked_until: &str) -> Result<()> {
  let now = now_rfc3339();
  let updated = sqlx::query("UPDATE users SET locked_until = ?, updated_at = ? WHERE id = ?")
    .bind(locked_until)
    .bind(&now)
    .bind(user_id)
    .execute(pool)
    .await
    .context("failed to set locked_until")?;
  if updated.rows_affected() == 0 {
    anyhow::bail!("user not found: {user_id}");
  }
  Ok(())
}

/// Resets `failed_attempts` to 0 and clears `locked_until`.
pub async fn reset_failed_attempts(pool: &SqlitePool, user_id: i64) -> Result<()> {
  let now = now_rfc3339();
  let updated = sqlx::query(
    "UPDATE users SET failed_attempts = 0, locked_until = NULL, updated_at = ? \
        WHERE id = ?",
  )
  .bind(&now)
  .bind(user_id)
  .execute(pool)
  .await
  .context("failed to reset failed_attempts")?;
  if updated.rows_affected() == 0 {
    anyhow::bail!("user not found: {user_id}");
  }
  Ok(())
}

/// Inserts a row into `auth_audit_log`.
///
/// `details` is optional free-form context (e.g. the lockout trigger). It
/// MUST NOT contain the password or any other secret.
pub async fn insert_audit(
  pool: &SqlitePool,
  event_type: AuthEventType,
  username: &str,
  success: bool,
  details: Option<&str>,
) -> Result<()> {
  let now = Utc::now().to_rfc3339();
  sqlx::query(
    "INSERT INTO auth_audit_log (event_type, username, success, occurred_at, details) \
        VALUES (?, ?, ?, ?, ?)",
  )
  .bind(event_type.as_str())
  .bind(username)
  .bind(i64::from(success))
  .bind(&now)
  .bind(details)
  .execute(pool)
  .await
  .context("failed to insert audit log row")?;
  Ok(())
}
