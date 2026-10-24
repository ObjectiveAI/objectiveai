//! Edit request data.
//!
//! What a caller hands the daemon to change an outgoing provider: its
//! address, and the [`Patch`](crate::daemon::patch::Patch) to its
//! [`Editable`] member, the mode.

mod editable;
mod frame;

pub use editable::*;
pub use frame::*;
