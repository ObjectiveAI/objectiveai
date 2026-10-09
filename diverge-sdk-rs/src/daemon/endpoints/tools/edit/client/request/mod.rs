//! Edit request data.
//!
//! What a caller hands the daemon to change a tool: which tool, and
//! the [`Edit`](crate::daemon::edit::Edit) shared with the agents
//! edit, flattened in — its name, its account, its mounts, each as it is to be or absent.

mod frame;

pub use frame::*;
