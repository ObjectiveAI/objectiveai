//! The content a connector streams for a write.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. A write's content is the same content
//! wherever a write happens, and
//! [`write_bytes`](crate::shared::container::write_bytes)
//! already says how the stream ends, what failing it means, and what
//! abandoning it means.

mod frame;

pub use frame::*;
