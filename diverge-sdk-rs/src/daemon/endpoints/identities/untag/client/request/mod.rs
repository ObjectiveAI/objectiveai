//! Untag request data.
//!
//! What a caller hands the daemon to take tags off an identity: its issuer, its subject and
//! the tags, and nothing else.

mod frame;

pub use frame::*;
