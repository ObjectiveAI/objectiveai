//! The content a caller streams for a write.
//!
//! Not an alias. The bytes are
//! [`write_bytes`](crate::shared::container::write_bytes)'s, and that
//! module says how a stream of them ends — but whether this channel
//! can also carry a failure is this endpoint's question, so [`Frame`]
//! is its own enum rather than a name for somebody else's.

mod frame;

pub use frame::*;
