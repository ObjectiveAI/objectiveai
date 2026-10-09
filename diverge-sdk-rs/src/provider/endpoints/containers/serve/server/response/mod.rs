//! Container serve response data.
//!
//! [`Frame`] is what comes back on channel `0`, once: the subtree is
//! served, or a failure.

mod frame;

pub use frame::*;
