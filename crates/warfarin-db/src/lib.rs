//! Data access layer for warfarin-care.
//!
//! Hosts the read-only HOSxP MySQL queries (`mysql`) and the read/write
//! local SQLite persistence (`sqlite`), plus the cloud-sync row models
//! (`sync_models`). This crate depends on `warfarin-core` for shared
//! domain models and pure helpers, but it must NEVER depend on Tauri.

pub mod mysql;
pub mod sqlite;
pub mod sync_models;
