//! The channels a server opens on a client.
//!
//! One, and it is a connection the provider cannot make itself: the
//! agent runs beside it, and the MCP servers live with the client.
//! [`Frame`] is what opens one, and it is the only frame here.
//!
//! What it carries is [`http::request`](crate::shared::http::request),
//! which is not here and should not be: a tunneled HTTP request is the
//! same request whatever protocol rides it.

mod frame;

pub use frame::*;
