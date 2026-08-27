//! Drug interaction checking engine.
//!
//! This module provides a pure function to detect interactions between
//! a patient's current medications and warfarin. It contains no I/O,
//! no database access, and no Tauri coupling - fully unit-testable.

mod checker;

pub use checker::check;
