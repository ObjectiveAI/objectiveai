//! Logs request data.
//!
//! What a caller hands the daemon to read an agent's log: the name,
//! and every way of narrowing what comes back, each optional. There
//! is nothing to establish and nothing to resume, which is why this
//! is one type and not a module of them.

mod frame;

pub use frame::*;
