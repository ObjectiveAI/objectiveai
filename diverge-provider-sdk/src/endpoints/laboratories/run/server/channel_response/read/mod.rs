//! The file bytes a provider relays out of the container.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. A read answer is the same answer
//! wherever it is asked for, and
//! [`read::response`](crate::shared::container::read::response)
//! already says what one is and how a stream of them ends.

mod frame;

pub use frame::*;
