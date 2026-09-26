//! Volume write response data.
//!
//! [`Frame`] is what comes back on channel `0`, once the content
//! channel has finished: the file landed, or a failure.

mod frame;

pub use frame::*;
