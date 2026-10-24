//! Edit request data.
//!
//! What a caller hands the daemon to change a daemon record: its name,
//! and the [`Patch`](crate::daemon::patch::Patch) to its [`Editable`]
//! member, the links.

mod editable;
mod frame;

pub use editable::*;
pub use frame::*;
