//! What the server sends back on a begin, one module per kind of
//! channel the proxy opens.
//!
//! Every one answers the CONTAINER, relayed: [`postgres`] is what its
//! database said, and the five `mcp_*` what the caller's MCP servers answered its tool
//! calls with — each an alias of the shape [`shared`](crate::shared)
//! defines.

pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
pub mod postgres;
