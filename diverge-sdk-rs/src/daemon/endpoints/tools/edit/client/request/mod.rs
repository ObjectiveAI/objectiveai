//! Edit request data.
//!
//! What a caller hands the daemon to change a tool's mounts: its
//! name, and the three lists of mounts as they are to be. The mount
//! types are the agents [`create`](crate::daemon::endpoints::agents::create)'s
//! own, as the tools [`create`](crate::daemon::endpoints::tools::create)
//! names them.

mod frame;

pub use frame::*;
