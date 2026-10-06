//! The answers the proxy sends on the channels the server opened.
//!
//! [`daemon`] is what the program sent on a daemon connection,
//! [`postgres`] is what the container's driver wrote on a database
//! connection, [`schema`] what its arguments may be. The five `mcp_*`
//! are what the container's own MCP server answered — the family's own,
//! each an alias of the shape [`shared::mcp`](crate::shared::mcp)
//! defines.

pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
pub mod daemon;
pub mod postgres;
pub mod schema;
