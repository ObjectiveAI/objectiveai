//! The seven requests over roles, served.
//!
//! One handler per request, each the shape [`serve`](super) states;
//! the twins of [`accounts`](super::accounts), with no live state to
//! ask about and a name that never changes.

pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
