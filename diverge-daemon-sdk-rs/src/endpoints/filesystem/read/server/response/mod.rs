//! Filesystem read response data.
//!
//! [`Frame`] is what comes back on channel `0`, as many times as the
//! file has pieces — one piece, or a failure.

mod frame;

pub use frame::*;
