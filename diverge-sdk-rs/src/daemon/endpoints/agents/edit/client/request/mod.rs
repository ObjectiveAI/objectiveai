//! Edit request data.
//!
//! What a caller hands the daemon to change an agent: which agent, and
//! the [`Patch`](crate::daemon::patch::Patch) to its [`Editable`]
//! members — its name, its mounts.

mod editable;
mod frame;

pub use editable::*;
pub use frame::*;
