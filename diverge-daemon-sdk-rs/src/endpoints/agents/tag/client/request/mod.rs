//! Tag request data.
//!
//! What a caller hands the daemon to spawn an agent under a tag: the
//! container, and the tag. There is nothing to establish and nothing
//! to resume, which is why this is one type and not a module of them.

mod frame;

pub use frame::*;
