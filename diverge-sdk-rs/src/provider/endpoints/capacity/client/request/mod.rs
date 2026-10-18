//! Capacity request data.
//!
//! [`Frame`] and nothing else, because a capacity request has nothing
//! to say beyond which request it is.

mod frame;

pub use frame::*;
