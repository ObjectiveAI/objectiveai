//! The tool container's own MCP server, called.
//!
//! A tool container's entrypoint is an MCP server on the loopback —
//! Streamable HTTP at `/mcp`, on the port
//! [`port()`](diverge_sdk::container_proxy::inside::port) reads, beside the
//! `/register`, `/secrets` and `/secrets/collect` that
//! [`program`](crate::program) makes, what the arguments may be being
//! the image's `/DIVERGE.md`, which [`info`](crate::info) reads from
//! the root and asks of no program —
//! and each channel the server opens on a tool container's begin
//! scope is one exchange with it, answered in the wire's own frame.
//! The proxy opens these sessions only for a tool container's begin,
//! which is the server's to open. The rules:
//!
//! 1. Every exchange names its caller's session
//!    ([`session`](diverge_sdk::shared::mcp::session)), and goes to
//!    that session's client ([`Sessions`]). A session is made on its
//!    caller's first exchange of any kind, notifications included, and
//!    its client dialled then: an `initialize` carrying the caller
//!    under `_meta`, so the server knows from the session's first
//!    message who is on the other end.
//! 2. A notifications channel hears what the server sends on its own
//!    session, and nothing sent on another.
//! 3. A session lives as long as the proxy; its client is re-dialled,
//!    with the same `initialize`, when its transport dies. Nothing
//!    else of the server's is kept.
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
mod session;
mod sessions;

pub use activity::*;
pub use call_tool::*;
pub use client::*;
pub use feed::*;
pub use list_resources::*;
pub use list_tools::*;
pub use notifications::*;
pub use read_resource::*;
pub use session::*;
pub use sessions::*;
