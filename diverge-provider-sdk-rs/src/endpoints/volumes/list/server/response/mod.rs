//! Volume listing response data.
//!
//! [`Frame`] is what comes back on channel `0` — the volumes, or a
//! failure to list them — and [`Volume`] is what the first is made
//! of.

mod frame;
mod volume;

pub use frame::*;
pub use volume::*;
