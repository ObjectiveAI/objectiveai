//! Whether a write landed.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. See
//! [`write_path::response`](crate::shared::container::write_path::response)
//! for why a bare finish would not have been enough.

mod frame;

pub use frame::*;
