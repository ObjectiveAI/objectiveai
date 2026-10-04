//! Edit request data.
//!
//! What a caller hands the daemon to change an agent: which agent,
//! and the [`Edit`](crate::daemon::edit::Edit) shared with the tools
//! edit, flattened in — its name, its mounts, the daemon's tools, its
//! deployer, each as it is to be or absent.

mod frame;

pub use frame::*;
