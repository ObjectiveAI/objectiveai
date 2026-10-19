//! The seven requests over accounts, served.
//!
//! One handler per request, each the shape [`serve`](super) states.
//! [`named_roles`] is what a create and an edit share: the roles a
//! request names, resolved and judged — each one the daemon has, or
//! `NoRole`; each one the caller holds the `grant` grant over, or
//! `Forbidden`.

mod named_roles;

pub use named_roles::*;

pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
