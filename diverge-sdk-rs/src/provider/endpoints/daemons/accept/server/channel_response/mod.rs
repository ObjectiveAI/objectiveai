//! What the provider answers the daemon's half of a connection with:
//! the connector's client frames, one per channel response. See
//! [`Frame`].

mod frame;

pub use frame::*;
