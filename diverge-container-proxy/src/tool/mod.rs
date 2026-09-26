//! The tool container's own MCP server, called.
//!
//! A tool container's entrypoint is an MCP server on the loopback —
//! Streamable HTTP at `/mcp`, on the port
//! [`port()`](diverge_sdk::container_proxy::inside::port) reads, beside the
//! `/register` and `/schema` that [`program`](crate::program) makes —
//! and each channel the server opens on a tool container's begin
//! scope is one exchange with it, answered in the wire's own frame.
//! One MCP client, dialled on the first exchange and kept while its
//! transport lives; the notifications it hears are fanned out to
//! every notifications channel open at the time. Nothing else of the
//! server's is kept. The proxy opens this client only for a tool
//! container's begin, which is the server's to open.

mod call_tool;
mod client;
mod list_resources;
mod list_tools;
mod notifications;
mod read_resource;

pub use call_tool::*;
pub use client::*;
pub use list_resources::*;
pub use list_tools::*;
pub use notifications::*;
pub use read_resource::*;
