//! Filesystem filetree response data.
//!
//! [`Frame`] is what comes back on channel `0`, again and again — one
//! filetree frame, or a failure, last.

mod frame;

pub use frame::*;
