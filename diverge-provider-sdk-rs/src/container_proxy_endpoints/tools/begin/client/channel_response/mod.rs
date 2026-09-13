//! What the server sends back on a begin, one module per kind of
//! channel the proxy opens.
//!
//! Every one answers the CONTAINER, relayed: [`postgres`] is what its
//! database said, [`command`] the items its command produced, the
//! five `vault_*` what its vault answered, and the five `mcp_*` what
//! the caller's MCP servers answered its tool calls with — each an
//! alias of the shape [`shared`](crate::shared) defines.

pub mod command;
pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
pub mod postgres;
pub mod vault_delete;
pub mod vault_get;
pub mod vault_lock;
pub mod vault_set;
pub mod vault_unlock;
