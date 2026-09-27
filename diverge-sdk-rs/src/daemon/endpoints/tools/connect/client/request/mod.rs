//! Connect request data.
//!
//! What a caller hands the daemon to connect to a tool container
//! somebody else runs, under a name: where the container is — its
//! provider, its id, the authorization its runner judges — and the
//! name. Nothing the container is made from, which is its runner's.

mod frame;

pub use frame::*;
