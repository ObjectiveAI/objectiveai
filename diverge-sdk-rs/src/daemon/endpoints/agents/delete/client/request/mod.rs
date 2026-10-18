//! Delete request data.
//!
//! What a caller hands the daemon to delete an agent: its name, and
//! nothing else. There is nothing to establish and nothing to resume,
//! which is why this is one type and not a module of them.

mod frame;

pub use frame::*;
