//! Data access layer for warfarin-care.
//!
//! Hosts the read-only `HOSxP` `MySQL` queries (`mysql`) and the read/write
//! local `SQLite` persistence (`sqlite`), plus the cloud-sync row models
//! (`sync_models`). This crate depends on `warfarin-core` for shared
//! domain models and pure helpers, but it must NEVER depend on Tauri.
//!
//! # Error convention
//!
//! Every public function that returns `Result` propagates `sqlx::Error`
//! (connection, query, or row-decode failures) and, where JSON columns are
//! decoded, `serde_json::Error` - both wrapped in `anyhow::Error`. That
//! contract is uniform across the crate, so per-function `# Errors`
//! sections would only duplicate this note. `clippy::missing_errors_doc`
//! is therefore relaxed at the crate level; functions whose error behaviour
//! is non-obvious still carry an explicit `# Errors` section.

#![warn(clippy::pedantic)]
// Uniform sqlx/serde_json → anyhow error contract; see crate docs above.
#![allow(clippy::missing_errors_doc)]

pub mod auth_repository;
pub mod auth_service;
pub mod backup;
pub mod mysql;
pub mod sqlite;
pub mod sync_models;

#[cfg(test)]
mod migration_tests {
  use std::fs;

  /// sqlx checksums migration files over their raw bytes at compile time, so
  /// a line-ending flip breaks every existing database. `.gitattributes`
  /// pins SQL to CRLF; this test catches a file created with bare LF before
  /// it can be applied anywhere.
  #[test]
  fn migration_files_use_crlf_line_endings() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
    let entries = fs::read_dir(dir).expect("migrations directory must exist");
    for entry in entries.flatten() {
      let path = entry.path();
      if path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
        continue;
      }
      let bytes = fs::read(&path).expect("migration must be readable");
      let mut previous_was_cr = false;
      for &byte in &bytes {
        assert!(
          !(byte == b'\n' && !previous_was_cr),
          "{} uses a bare LF; sqlx checksums require CRLF (see .gitattributes)",
          path.display()
        );
        previous_was_cr = byte == b'\r';
      }
    }
  }
}
