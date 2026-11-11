//! Daemons list response data.
//!
//! [`Frame`] is what comes back on channel `0`: a [`Listed`]
//! added or removed, or the word that the listing is whole.

mod listed;
mod frame;

pub use listed::*;
pub use frame::*;
