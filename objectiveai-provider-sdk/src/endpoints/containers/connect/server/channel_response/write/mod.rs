//! Whether a write landed.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. See
//! [`write_path::response`](crate::shared::container::write_path::response)
//! for what its absence means, and why it spends a byte anyway.

mod frame;

pub use frame::*;
