//! Data access layer for warfarin-care.
//!
//! Hosts the read-only HOSxP MySQL queries and the read/write local SQLite
//! persistence (clinic tracking). This crate depends on `warfarin-core` for
//! shared domain models and pure helpers, but it must NEVER depend on Tauri.
