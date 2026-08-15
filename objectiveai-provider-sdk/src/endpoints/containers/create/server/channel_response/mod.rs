//! The answers a provider sends on the channels a caller
//! opened.
//!
//! [`mcp`] is what the container's MCP server said, [`read`] is a
//! file's bytes, and [`write_path`] is whether one landed.

pub mod mcp;
pub mod read;
pub mod write_path;
