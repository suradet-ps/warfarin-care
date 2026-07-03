//! Pure domain logic for the warfarin-care clinic management system.
//!
//! This crate contains business rules, dose calculation, INR/TTR algorithms,
//! encryption helpers, and data models that are independent of any I/O or
//! Tauri runtime. It is intentionally kept dependency-light so it can be
//! unit-tested in isolation and reused outside the desktop app.

#![warn(clippy::pedantic)]
// `aes-gcm 0.11` exposes `Nonce::from_slice` as a deprecated alias for the
// `TryFrom` impl on `GenericArray`. The upstream fix is on a 0.12+ release
// line; until we upgrade, silence the deprecation so `clippy -D warnings`
// stays green.
#![allow(deprecated)]

pub mod auth;
pub mod dose;
pub mod encrypt;
pub mod models;
pub mod pills;
pub mod screening;
