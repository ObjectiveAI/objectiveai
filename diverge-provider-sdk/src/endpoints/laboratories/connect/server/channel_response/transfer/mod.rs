//! Whether a transfer landed.
//!
//! Not an alias. What rides this channel is
//! [`transfer`](crate::shared::container::transfer)'s, and that module
//! says what it means — but whether this channel can also carry a
//! failure is this endpoint's question, so [`Frame`] is its own enum
//! rather than a name for somebody else's.

mod frame;

pub use frame::*;
