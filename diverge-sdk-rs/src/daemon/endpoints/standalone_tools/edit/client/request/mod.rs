//! Edit request data.
//!
//! What a caller hands the daemon to change a tool: which tool, and
//! the [`Patch`](crate::daemon::patch::Patch) to its [`Editable`]
//! members — its name, its account, its mounts.

mod editable;
mod frame;

pub use editable::*;
pub use frame::*;
