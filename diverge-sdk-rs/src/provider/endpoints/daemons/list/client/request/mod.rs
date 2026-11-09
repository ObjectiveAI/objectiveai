//! Daemons list request data.
//!
//! [`Frame`] and nothing else, because a daemons list request has nothing
//! to say beyond which request it is.

mod frame;

pub use frame::*;
