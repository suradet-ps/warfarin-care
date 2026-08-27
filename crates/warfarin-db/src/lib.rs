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
pub mod mysql;
pub mod sqlite;
pub mod sync_models;
