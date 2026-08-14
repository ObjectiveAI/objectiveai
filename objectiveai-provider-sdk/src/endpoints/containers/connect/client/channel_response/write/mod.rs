//! The content a connector streams for a write.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. The bytes are the same bytes
//! wherever a write happens, and
//! [`write::bytes`](crate::shared::container::write::bytes)
//! already says how the stream ends and what abandoning it
//! means.

mod frame;

pub use frame::*;
