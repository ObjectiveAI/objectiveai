//! Volume edit-capacity response data.
//!
//! [`Frame`] is what comes back on channel `0` — the number of bytes,
//! or a failure to say.

mod frame;

pub use frame::*;
