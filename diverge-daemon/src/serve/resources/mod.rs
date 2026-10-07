//! The nine requests over resources, served.
//!
//! One handler per request, each the shape [`serve`](crate::serve)
//! states; the bytes are [`content`](crate::content)'s. Whether some
//! container mounts a resource — what a delete answers `InUse` for and
//! a filter asks as `in_use` — is [`in_use`], false until containers
//! exist. A transfer lands nowhere yet: its destinations are
//! containers and volumes, which do not exist, so it answers
//! `NoDestination` for them and `IntoResource` for a resource, as the
//! wire has it.

mod in_use;

pub use in_use::*;

pub mod delete;
pub mod download;
pub mod filetree;
pub mod get;
pub mod list;
pub mod tag;
pub mod transfer;
pub mod untag;
pub mod upload;
