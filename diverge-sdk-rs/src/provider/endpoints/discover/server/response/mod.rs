//! Discover listing response data.
//!
//! [`Frame`] is what comes back on channel `0`: an identity added or
//! removed, or the word that the listing is whole.

mod frame;

pub use frame::*;
