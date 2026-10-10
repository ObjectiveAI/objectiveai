//! Outgoing providers: the ones the daemon dials.
//!
//! An outgoing provider is an address and a [`Mode`]: how the daemon
//! authenticates to it, unbrokered with a credential of the client's.
//! Its identity is the address, as
//! [`Identity`](crate::daemon::endpoints::agents::logs::server::response::Identity)
//! states. [`add`] names one; [`get`] answers one as a list would;
//! [`list`] lists them, narrowed, with their tags; [`delete`] forgets
//! one no container is pinned to; [`edit`] replaces its mode, which is
//! how a credential rotates; [`tag`] and [`untag`] change its tags,
//! which are the caller's.

mod kind;
mod mode;

pub use kind::*;
pub use mode::*;

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
