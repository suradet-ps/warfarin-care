//! Automatic pre-migration snapshots of the local `SQLite` database.
//!
//! The clinic's ledger is the only copy of its data. Before any pending
//! migration rewrites the schema, [`backup_before_migration`] snapshots the
//! database with `VACUUM INTO`, which produces a consistent copy even while
//! a WAL is present. Snapshots live next to the database file as
//! `warfarin.db.bak-<timestamp>`, and [`MAX_BACKUPS`] of them are kept.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;

/// Number of pre-migration snapshots kept next to the database.
pub const MAX_BACKUPS: usize = 5;

/// Writes a pre-migration snapshot when the database has pending migrations.
///
/// Returns the snapshot path when one was written, or `None` for a brand new
/// or already up-to-date database. `had_data` tells the function whether the
/// database file contained data before the pool was opened, so a freshly
/// created file does not produce a useless snapshot.
///
/// # Errors
///
/// Propagates filesystem and `SQLite` errors. [`crate::sqlite::init_pool`]
/// treats a failed snapshot as fatal: no schema change happens without one.
pub async fn backup_before_migration(
  pool: &SqlitePool,
  db_path: &Path,
  migrator: &Migrator,
  had_data: bool,
) -> Result<Option<PathBuf>> {
  if !had_data || !has_pending_migrations(pool, migrator).await? {
    return Ok(None);
  }

  let backup = backup_path(db_path);
  let target = backup.to_string_lossy().into_owned();
  sqlx::query("VACUUM INTO ?")
    .bind(target)
    .execute(pool)
    .await
    .with_context(|| format!("failed to write backup at {}", backup.display()))?;

  prune_backups(db_path);

  Ok(Some(backup))
}

/// Returns `true` when the migrator has versions the database has not applied.
pub async fn has_pending_migrations(pool: &SqlitePool, migrator: &Migrator) -> Result<bool> {
  let applied: Vec<i64> =
    match sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success = 1")
      .fetch_all(pool)
      .await
    {
      Ok(versions) => versions,
      // No migration bookkeeping yet: the database predates sqlx migrations
      // or was just created, so every migration counts as pending.
      Err(sqlx::Error::Database(error)) if error.message().contains("no such table") => {
        return Ok(true);
      }
      Err(error) => return Err(error.into()),
    };

  Ok(
    migrator
      .iter()
      .any(|migration| !applied.contains(&migration.version)),
  )
}

fn backup_path(db_path: &Path) -> PathBuf {
  let file_name = db_path.file_name().map_or_else(
    || String::from("warfarin.db"),
    |name| name.to_string_lossy().into_owned(),
  );
  let stamp = Utc::now().format("%Y%m%d-%H%M%S");
  db_path.with_file_name(format!("{file_name}.bak-{stamp}"))
}

/// Removes the oldest snapshots until at most [`MAX_BACKUPS`] remain.
fn prune_backups(db_path: &Path) {
  let Some(dir) = db_path.parent() else {
    return;
  };
  let Some(file_name) = db_path.file_name().and_then(|name| name.to_str()) else {
    return;
  };
  let prefix = format!("{file_name}.bak-");
  let Ok(entries) = fs::read_dir(dir) else {
    return;
  };

  let mut backups: Vec<PathBuf> = entries
    .flatten()
    .map(|entry| entry.path())
    .filter(|path| {
      path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(&prefix))
    })
    .collect();
  backups.sort();

  while backups.len() > MAX_BACKUPS {
    let oldest = backups.remove(0);
    let _ = fs::remove_file(oldest);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::sqlite::SqlitePoolOptions;
  use uuid::Uuid;

  fn temp_db_path() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("warfarin-backup-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    dir.join("warfarin.db")
  }

  async fn connect(path: &Path) -> SqlitePool {
    let url = format!("sqlite://{}?mode=rwc", path.display());
    SqlitePoolOptions::new()
      .max_connections(1)
      .connect(&url)
      .await
      .unwrap()
  }

  fn cleanup(db_path: &Path) {
    let _ = fs::remove_dir_all(db_path.parent().unwrap());
  }

  #[tokio::test]
  async fn snapshots_only_when_migrations_are_pending() {
    let db_path = temp_db_path();
    let pool = connect(&db_path).await;
    let migrator = sqlx::migrate!("./migrations");
    migrator.run(&pool).await.unwrap();

    assert!(!has_pending_migrations(&pool, &migrator).await.unwrap());
    assert!(
      backup_before_migration(&pool, &db_path, &migrator, true)
        .await
        .unwrap()
        .is_none()
    );

    // Simulate a pending migration by dropping the last applied record.
    sqlx::query(
      "DELETE FROM _sqlx_migrations WHERE version = (SELECT MAX(version) FROM _sqlx_migrations)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(has_pending_migrations(&pool, &migrator).await.unwrap());

    let backup = backup_before_migration(&pool, &db_path, &migrator, true)
      .await
      .unwrap()
      .expect("pending migrations must produce a snapshot");
    assert!(backup.exists());

    // The snapshot must be a readable database with the migration bookkeeping
    // intact, so it can be restored in place of the live file.
    let restored = connect(&backup).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
      .fetch_one(&restored)
      .await
      .unwrap();
    assert!(count > 0);

    drop(restored);
    drop(pool);
    cleanup(&db_path);
  }

  #[tokio::test]
  async fn skips_brand_new_databases() {
    let db_path = temp_db_path();
    let pool = connect(&db_path).await;
    let migrator = sqlx::migrate!("./migrations");

    assert!(
      backup_before_migration(&pool, &db_path, &migrator, false)
        .await
        .unwrap()
        .is_none()
    );

    drop(pool);
    cleanup(&db_path);
  }

  #[test]
  fn retention_keeps_only_the_latest_snapshots() {
    let db_path = temp_db_path();
    fs::write(&db_path, b"live database").unwrap();
    for minute in 0..(MAX_BACKUPS + 3) {
      let name = format!("warfarin.db.bak-20260101-0000{minute:02}");
      fs::write(db_path.with_file_name(name), b"snapshot").unwrap();
    }

    prune_backups(&db_path);

    let remaining = fs::read_dir(db_path.parent().unwrap())
      .unwrap()
      .flatten()
      .filter(|entry| entry.file_name().to_string_lossy().contains(".bak-"))
      .count();
    assert_eq!(remaining, MAX_BACKUPS);
    assert!(
      !db_path
        .with_file_name("warfarin.db.bak-20260101-000000")
        .exists()
    );
    assert!(
      db_path
        .with_file_name(format!("warfarin.db.bak-20260101-00000{}", MAX_BACKUPS + 2))
        .exists()
    );

    cleanup(&db_path);
  }
}
