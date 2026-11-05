//! Discover listing response data.
//!
//! [`Frame`] is what comes back on channel `0`: a [`Discoverable`]
//! added or removed, or the word that the listing is whole.

mod discoverable;
mod frame;

pub use discoverable::*;
pub use frame::*;
