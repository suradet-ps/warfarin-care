//! Domain models shared across the warfarin-care application.
//!
//! All types here are plain `serde` structs with no I/O or runtime coupling,
//! so they can be serialized over Tauri IPC and reused by the data layer.

pub mod alert;
pub mod appointment;
pub mod dispensing;
pub mod inr;
pub mod interaction;
pub mod outcome;
pub mod patient;
pub mod visit;
