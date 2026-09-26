//! Message request data.
//!
//! What a caller hands the daemon to send an agent a message: the
//! agent's name, and the content. There is nothing to establish and
//! nothing to resume, which is why this is one type and not a module
//! of them.

mod frame;

pub use frame::*;
