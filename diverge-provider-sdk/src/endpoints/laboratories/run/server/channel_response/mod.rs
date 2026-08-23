//! The answers a provider sends on the channels a caller
//! opened.
//!
//! [`mcp`] is what the container's MCP server said, [`read`] is a
//! file's bytes, [`write_path`] is whether one landed, and
//! [`transfer`] is whether one moved between containers without ever
//! becoming bytes.

pub mod mcp;
pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
pub mod read;
pub mod transfer;
pub mod write_path;
