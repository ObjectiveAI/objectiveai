//! The `/tool/*` paths: the tool container's own MCP server, called.
//!
//! A tool container's entrypoint is an MCP server on the loopback —
//! Streamable HTTP at `/mcp`, on the port the SDK's
//! [`agent::port()`](diverge_provider_sdk::container_proxy::agent::port)
//! reads — and each path here is one exchange with it, made when the
//! provider's server opens the path and answered in the wire's own
//! frame. One MCP client, dialled on the first opening and kept while
//! its transport lives; the notifications it hears are fanned out to
//! every `/tool/notifications` open at the time. Nothing else of the
//! server's is kept. The proxy cannot tell a tool container from an
//! agent container, and does not try; the provider's server opens
//! these paths only on a tool container.

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
