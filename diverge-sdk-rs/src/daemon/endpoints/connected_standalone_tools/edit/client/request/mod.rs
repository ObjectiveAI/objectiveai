//! Edit request data.
//!
//! What a caller hands the daemon to change a connected tool: which
//! tool, and the [`Patch`](crate::daemon::patch::Patch) to its
//! [`Editable`] members — its name.

mod editable;
mod frame;

pub use editable::*;
pub use frame::*;
