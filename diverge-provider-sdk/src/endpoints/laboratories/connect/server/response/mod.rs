//! What a provider sends back on a connection.
//!
//! The container's filesystem, and nothing else — see [`Frame`].
//!
//! Note what is absent. A run's answer carries the container's id; a
//! connection's does not, because a connector supplied it to get here.
//! Nothing is minted by joining. It carries no count of the other
//! connectors either, and no word when one comes or goes: a connector
//! joined a container, not a room.

mod frame;

pub use frame::*;
