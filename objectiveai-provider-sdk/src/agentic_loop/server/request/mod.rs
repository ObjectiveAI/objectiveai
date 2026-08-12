//! The requests a server makes of its client.
//!
//! Both are the same ask in different clothes: a connection the
//! provider cannot make itself. The agent runs beside it; the MCP
//! servers and the database live with the client. [`Frame`] is what
//! opens one, and [`mcp`] is what an MCP one carries.

pub mod mcp;

mod frame;

pub use frame::*;
