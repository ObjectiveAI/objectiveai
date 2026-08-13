//! The channels a server opens on a client.
//!
//! Both are the same ask in different clothes: a connection the
//! provider cannot make itself. The agent runs beside it; the MCP
//! servers and the database live with the client. [`Frame`] is what
//! opens one, and it is the only frame here.
//!
//! What an MCP one carries is
//! [`http::request`](crate::http::request), which is not here and
//! should not be: a tunneled HTTP request is the same request whatever
//! protocol rides it.

mod frame;

pub use frame::*;
