//! Untag request data.
//!
//! What a caller hands the daemon to take tags off an incoming credential: its name and
//! the tags, and nothing else.

mod frame;

pub use frame::*;
