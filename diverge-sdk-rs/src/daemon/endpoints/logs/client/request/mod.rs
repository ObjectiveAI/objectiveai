//! Logs request data.
//!
//! What a caller hands the daemon to read a log: the [`Subject`], every
//! way of narrowing what comes back, and a cap on how much, each
//! optional. [`ItemType`] is the vocabulary of one of those
//! narrowings, the kind of item.

mod frame;
mod item_type;
mod subject;

pub use frame::*;
pub use item_type::*;
pub use subject::*;
