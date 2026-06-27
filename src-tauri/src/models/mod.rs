//! Local re-export for sync models that derive `sqlx::FromRow`.
//!
//! The rest of the domain models now live in `warfarin-core`. Only the sync
//! row types remain here because they are coupled to `sqlx`; they will move
//! to `warfarin-db` in a follow-up commit.

pub mod sync;
