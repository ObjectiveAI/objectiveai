//! Edit request data.
//!
//! What a caller hands the daemon to change a role: its name, and the
//! [`Patch`](crate::daemon::patch::Patch) to its [`Editable`] members.

mod editable;
mod frame;

pub use editable::*;
pub use frame::*;
