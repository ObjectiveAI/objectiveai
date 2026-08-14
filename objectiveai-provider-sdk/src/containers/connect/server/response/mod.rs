//! What a provider sends back on a connection.
//!
//! The container's filesystem, and how many connectors are on it. In
//! no particular order — see [`Frame`].
//!
//! Note what is absent. A creation's answer carries the container's
//! id; a connection's does not, because a connector supplied it to get
//! here. Nothing is minted by joining.

mod frame;

pub use frame::*;
