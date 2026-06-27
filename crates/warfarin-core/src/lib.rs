//! Pure domain logic for the warfarin-care clinic management system.
//!
//! This crate contains business rules, dose calculation, INR/TTR algorithms,
//! encryption helpers, and data models that are independent of any I/O or
//! Tauri runtime. It is intentionally kept dependency-light so it can be
//! unit-tested in isolation and reused outside the desktop app.

pub mod dose;
pub mod encrypt;
pub mod models;
pub mod pills;
pub mod screening;
