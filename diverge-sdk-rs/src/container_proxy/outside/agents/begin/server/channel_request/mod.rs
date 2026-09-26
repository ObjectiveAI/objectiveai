//! The channels the proxy opens on the server.
//!
//! Everything the container asks of the world outside — its database
//! connections, its commands, its vault, its tool calls outward — and
//! nothing the proxy asks on its own account. See [`Frame`].

mod frame;

pub use frame::*;
