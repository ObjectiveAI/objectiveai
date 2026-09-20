//! Create request data.
//!
//! What a caller hands the daemon to create an agent under a name: the
//! container, and the name. There is nothing to establish and nothing
//! to resume, which is why this is one type and not a module of them.

mod frame;

pub use frame::*;
