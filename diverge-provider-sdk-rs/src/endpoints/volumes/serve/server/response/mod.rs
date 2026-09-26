//! Volume serve response data.
//!
//! [`Frame`] is what comes back on channel `0`, once: the volume is
//! served, or a failure.

mod frame;

pub use frame::*;
