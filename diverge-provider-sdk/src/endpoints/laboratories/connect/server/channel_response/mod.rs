//! The answers a provider sends on the channels a connector
//! opened.
//!
//! [`read`] is a file's bytes, [`write_path`] is whether one landed,
//! and [`transfer`] is whether one moved between containers without
//! ever becoming bytes.
//!
//! The other five are what the container's MCP server said, one per
//! exchange, and each is an alias of the shape
//! [`shared::mcp`](crate::shared::mcp) defines — what a channel carries
//! is what MCP says it carries, whichever direction it runs.

pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
pub mod read;
pub mod transfer;
pub mod write_path;
