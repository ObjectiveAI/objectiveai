//! Tag request data.
//!
//! What a caller hands the daemon to put tags on a template: its id and
//! the tags, and nothing else. There is nothing to establish and
//! nothing to resume, which is why this is one type and not a module of
//! them.

mod frame;

pub use frame::*;
