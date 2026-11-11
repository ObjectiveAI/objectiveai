//! Listen response data.
//!
//! [`Frame`] is what comes back on channel `0`: a [`Message`], or a
//! refusal.

mod frame;
mod message;

pub use frame::*;
pub use message::*;
