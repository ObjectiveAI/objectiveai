//! An agent's log: everything its conversation has carried.
//!
//! The daemon keeps, for every agent it runs, every chunk the agent's
//! run streamed — the user parts of each message that landed, each with
//! who sent the message, everything the agent said, its tool calls and
//! their answers, its usage and notifications — every error the run
//! answered with, when the agent's container came up on a provider,
//! and which, when it began and ceased working, and when the container
//! went down and why — each under an index that counts up
//! and the time it was kept. The log outlives the run: it is the
//! daemon's, kept until the agent is deleted, and read by
//! [`logs`](crate::daemon::endpoints::logs).
//!
//! [`ItemWrapper`] is one entry of the log: its index, its time, and
//! the [`Item`] it holds — a [`User`] part with who sent it, another
//! chunk, an [`Error`], or the run's life as
//! [`lifecycle`](crate::daemon::lifecycle) states it: [`Deployed`] on
//! a provider, [`Active`] and [`Inactive`] again, and [`Undeployed`].
//! [`Provider`] is what a deployment says of the provider, and
//! [`Identity`] is who it is. [`ItemType`] names the kinds.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod active;
mod answer;
mod deployed;
mod error;
mod identity;
mod inactive;
mod item;
mod item_type;
mod item_wrapper;
mod structured_decisions_response;
mod undeployed;
mod user;
mod user_structured_decisions;

pub use active::*;
pub use answer::*;
pub use deployed::*;
pub use error::*;
pub use identity::*;
pub use inactive::*;
pub use item::*;
pub use item_type::*;
pub use item_wrapper::*;
pub use structured_decisions_response::*;
pub use undeployed::*;
pub use user::*;
pub use user_structured_decisions::*;

pub use crate::daemon::lifecycle::{Provider, Reason};
