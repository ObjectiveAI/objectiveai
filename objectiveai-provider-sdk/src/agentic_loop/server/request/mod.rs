//! The requests a server makes of its client.
//!
//! Both are the same ask in different clothes: a connection the
//! provider cannot make itself. The agent runs beside it; the MCP
//! servers and the database live with the client. [`Frame`] is what
//! opens one, and [`mcp`] is what an MCP one carries.
//!
//! [`mcp`] is [`crate::mcp::request`] under a shorter name, not a copy
//! of it. An MCP request is the same request wherever it is carried,
//! and the day a second scope carries one is the day two definitions
//! would start to differ.

pub use crate::mcp::request as mcp;

mod frame;

pub use frame::*;
