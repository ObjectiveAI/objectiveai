//! The channels a server opens on a connector.
//!
//! One, and only in answer to something the connector asked for
//! first — see [`Frame`].

mod frame;

pub use frame::*;
