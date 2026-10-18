//! The tool container's own MCP server, called.
//!
//! A tool container's entrypoint is an MCP server on the loopback —
//! Streamable HTTP at `/mcp`, on the port
//! [`port()`](diverge_sdk::container_proxy::inside::port) reads, beside the
//! `/register`, `/schema`, `/secrets` and `/secrets/collect` that
//! [`program`](crate::program) makes —
//! and each channel the server opens on a tool container's begin
//! scope is one exchange with it, answered in the wire's own frame.
//! One MCP client, dialled on the first exchange and kept while its
//! transport lives; the notifications it hears are fanned out to
//! every notifications channel open at the time. Nothing else of the
//! server's is kept. The proxy opens this client only for a tool
//! container's begin, which is the server's to open.
//!
//! The four exchanges that reach the server — list-tools,
//! list-resources, call-tool, read-resource — are the container's
//! activity: the first in flight is `Active` on the begin scope's
//! main stream, the last out is `Inactive` with the rotating secrets
//! collected from the program, and between an `Inactive` and the
//! next feed on the server's secrets channel an exchange waits
//! behind [`Activity`]'s gate — when the program takes any secret.
//! Notifications are not activity: a nudge, not an exchange.

mod activity;
mod call_tool;
mod client;
mod feed;
mod list_resources;
mod list_tools;
mod notifications;
mod read_resource;

pub use activity::*;
pub use call_tool::*;
pub use client::*;
pub use feed::*;
pub use list_resources::*;
pub use list_tools::*;
pub use notifications::*;
pub use read_resource::*;
