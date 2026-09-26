//! Volume filetree response data.
//!
//! [`Frame`] is what comes back on channel `0` — the tree, or a
//! failure to walk it.

mod frame;

pub use frame::*;
