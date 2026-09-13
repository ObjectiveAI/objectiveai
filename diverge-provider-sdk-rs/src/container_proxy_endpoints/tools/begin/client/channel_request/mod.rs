//! The channels the server opens on the proxy.
//!
//! The one every begin scope has — the server's half of a database
//! connection — and then this family's own. See [`Frame`].

mod frame;

pub use frame::*;
