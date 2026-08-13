//! The requests a server makes of its client.
//!
//! Both are the same ask in different clothes: a connection the
//! provider cannot make itself. The agent runs beside it; the MCP
//! servers and the database live with the client. [`Frame`] is what
//! opens one, and it is the only frame here.
//!
//! Which is why what an MCP one carries is named where it lives, at
//! [`mcp::request`](crate::mcp::request), rather than pulled under
//! this module. An MCP request is not a second kind of request frame
//! — it is the payload inside this one's
//! [`Mcp`](Frame::Mcp) variant.

mod frame;

pub use frame::*;
