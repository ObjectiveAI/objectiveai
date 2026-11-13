//! Structured decisions request data.
//!
//! What a caller hands the daemon to put a decision to an agent: the
//! agent's name, the content, and the questions. There is nothing to establish and
//! nothing to resume, which is why this is one type and not a module
//! of them.

mod frame;

pub use frame::*;
