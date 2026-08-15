//! The content a caller streams for a write.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. The bytes are the same bytes
//! wherever a write happens, and
//! [`write_bytes`](crate::shared::container::write_bytes)
//! already says how the stream ends and what abandoning it
//! means.

mod frame;

pub use frame::*;
