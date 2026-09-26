//! Logs request data.
//!
//! What a caller hands the daemon to read an agent's log: the name,
//! every way of narrowing what comes back, and a cap on how much,
//! each optional. [`ItemType`] is the vocabulary of one of those
//! narrowings, the kind of item.

mod frame;
mod item_type;

pub use frame::*;
pub use item_type::*;
