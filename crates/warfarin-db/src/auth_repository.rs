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
/// column's `UNIQUE` index is not enforced — we trim/normalize at the service
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
