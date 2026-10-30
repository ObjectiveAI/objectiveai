//! An agent's log: everything its conversation has carried.
//!
//! The daemon keeps, for every agent it runs, every chunk the agent's
//! run streamed — the user parts of each message that landed, each with
//! who sent the message, everything the agent said, its tool calls and
//! their answers, its usage and notifications — every error the run
//! answered with, and when the agent began running on a provider, and
//! which, and when it ceased to — each under an index that counts up
//! and the time it was kept. The log outlives the run: it is the
//! daemon's, kept until the agent is deleted, and read by
//! [`logs`](crate::daemon::endpoints::logs).
//!
//! [`ItemWrapper`] is one entry of the log: its index, its time, and
//! the [`Item`] it holds — a [`User`] part with who sent it, another
//! chunk, an [`Error`], or the agent going [`Active`] on a provider
//! and [`Inactive`] again. [`Provider`] is what those two say of the
//! provider, and [`Identity`] is who it is. [`ItemType`] names the
//! kinds.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod active;
mod error;
mod identity;
mod inactive;
mod item;
mod item_type;
mod item_wrapper;
mod provider;
mod user;

pub use active::*;
pub use error::*;
pub use identity::*;
pub use inactive::*;
pub use item::*;
pub use item_type::*;
pub use item_wrapper::*;
pub use provider::*;
pub use user::*;
