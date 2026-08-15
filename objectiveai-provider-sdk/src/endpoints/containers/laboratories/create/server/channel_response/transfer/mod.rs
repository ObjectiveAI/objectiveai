//! Whether a transfer landed.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. See
//! [`transfer::response`](crate::shared::container::transfer::response)
//! for what its absence means, and for the one thing it cannot say.

mod frame;

pub use frame::*;
