//! Edit request data.
//!
//! What a caller hands the daemon to change an agent's mounts: its
//! name, and the three lists of mounts as they are to be. The mount
//! types are the [`create`](crate::daemon::endpoints::agents::create)'s
//! own, named from there.

mod frame;

pub use frame::*;
