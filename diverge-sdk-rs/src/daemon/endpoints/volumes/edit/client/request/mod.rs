//! Edit request data.
//!
//! What a caller hands the daemon to change a volume: which one, and
//! the [`Patch`](crate::daemon::patch::Patch) to its [`Editable`]
//! members, the size and the mode.

mod editable;
mod frame;

pub use editable::*;
pub use frame::*;
