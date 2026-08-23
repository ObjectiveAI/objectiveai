//! The answers a client sends on the channels a server opened.
//!
//! Five of them are one MCP exchange each, and every one is an alias
//! of the shape [`shared::mcp`](crate::shared::mcp) defines: what a
//! channel carries here is what MCP says it carries, and MCP says the
//! same thing whichever direction the channel runs.
//!
//! [`mcp_list_tools`], [`mcp_list_resources`], [`mcp_call_tool`] and
//! [`mcp_read_resource`] answer once and finish; [`mcp_notifications`]
//! carries a frame per notification for as long as the channel lives.
//!
//! Every one is prefixed `mcp_`, because a channel a container opens is
//! not necessarily MCP's — a plugin's are a database and a command —
//! and a module called `call_tool` would only read as MCP's to somebody
//! who already knew.
//!
//! There was a sixth, and it was the older way: a whole HTTP exchange
//! tunneled, head and body and all. The five replaced everything it
//! could do, so it is gone, and nothing a client sends on this endpoint
//! is HTTP any more.
//!
//! Each stays a module in the path rather than being re-exported
//! upward. It is what tells five types called `Frame` apart, which was
//! the reason to keep the shape when there was only one of them.
//!
//! Note what is NOT here. The chunks of the loop itself are a response
//! too, but the SERVER sends those, so they live in
//! [`server::response`](crate::endpoints::agentic_loop::run::server::response). This
//! module is the other direction — a client answering what it was
//! asked for.

pub mod mcp_call_tool;
pub mod mcp_list_resources;
pub mod mcp_list_tools;
pub mod mcp_notifications;
pub mod mcp_read_resource;
