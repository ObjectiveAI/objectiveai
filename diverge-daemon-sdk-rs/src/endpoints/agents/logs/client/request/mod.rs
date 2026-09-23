//! Logs request data.
//!
//! What a caller hands the daemon to read an agent's log: the name,
//! every way of narrowing what comes back, and a cap on how much,
//! each optional. There
//! is nothing to establish and nothing to resume, which is why this
//! is one type and not a module of them.

mod frame;

pub use frame::*;
