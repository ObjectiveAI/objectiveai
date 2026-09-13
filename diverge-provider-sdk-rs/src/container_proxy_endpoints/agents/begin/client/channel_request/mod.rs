//! The channels the server opens on the proxy.
//!
//! The one every begin scope has — the server's half of a database
//! connection — and then this family's own. See [`Frame`]. The agent
//! a registration carries is [`Register`], defined here because it
//! rides no other wire.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod frame;
mod register;

pub use frame::*;
pub use register::*;
