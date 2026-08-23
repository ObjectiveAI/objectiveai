//! The answers a client sends on the channels a server opened.
//!
//! Four of them are one MCP exchange each — [`list_tools`],
//! [`list_resources`], [`call_tool`] and [`read_resource`] — and every
//! one is a result or an
//! [`ErrorData`](rmcp::ErrorData), with nothing around it.
//!
//! [`mcp`] is the fifth and is the older way: a whole HTTP exchange
//! tunneled, whose frame is an ALIAS of
//! [`http::response::Frame`](crate::shared::http::response::Frame),
//! because the head-then-body split is a fact about HTTP rather than
//! about this channel. It survives only for what the four cannot do
//! yet, which is an event stream.
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

pub mod call_tool;
pub mod list_resources;
pub mod list_tools;
pub mod mcp;
pub mod read_resource;
