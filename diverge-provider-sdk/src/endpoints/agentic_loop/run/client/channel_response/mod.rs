//! The answers a client sends on the channels a server opened.
//!
//! Five of them are one MCP exchange each. [`list_tools`],
//! [`list_resources`], [`call_tool`] and [`read_resource`] answer once
//! and finish; [`notifications`] carries a frame per notification for
//! as long as the channel lives. All five are a value or an
//! [`ErrorData`](rmcp::ErrorData), with nothing around it.
//!
//! [`mcp`] is the sixth and is the older way: a whole HTTP exchange
//! tunneled, whose frame is an ALIAS of
//! [`http::response::Frame`](crate::shared::http::response::Frame),
//! because the head-then-body split is a fact about HTTP rather than
//! about this channel. Nothing is left that only it can do.
//!
//! Each stays a module in the path rather than being re-exported
//! upward. It is what tells six types called `Frame` apart, which was
//! the reason to keep the shape when there was only one of them.
//!
//! Note what is NOT here. The chunks of the loop itself are a response
//! too, but the SERVER sends those, so they live in
//! [`server::response`](crate::endpoints::agentic_loop::run::server::response). This
//! module is the other direction — a client answering what it was
//! asked for.

pub mod call_tool;
pub mod list_resources;
pub mod list_tools;
pub mod mcp;
pub mod notifications;
pub mod read_resource;
