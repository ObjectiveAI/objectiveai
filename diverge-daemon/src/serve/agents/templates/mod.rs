//! The six requests over agent templates, served.
//!
//! One handler per request, each the shape [`serve`](crate::serve)
//! states. Whether some agent was made from a template — what a
//! delete answers `InUse` for and a filter asks as `in_use` — is
//! [`in_use`], false until agents exist.

mod in_use;

pub use in_use::*;

pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
