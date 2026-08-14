//! The answers a provider sends on the channels a connector
//! opened.
//!
//! Both come from inside the container: [`mcp`] is what its MCP server
//! said, [`read`] is a file's bytes.

pub mod mcp;
pub mod read;
