//! Accounts edit request data.
//!
//! What a caller hands the daemon for an accounts edit: the daemon,
//! the account's name, and the [`Patch`](crate::daemon::patch::Patch)
//! to its [`Editable`] member, the mode.

mod editable;
mod frame;

pub use editable::*;
pub use frame::*;
