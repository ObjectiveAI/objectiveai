//! The `/tool/*` paths: the tool container's own MCP server, reached.
//!
//! A tool container's entrypoint is an MCP server — Streamable HTTP
//! on the container's loopback, at `/mcp` on the port
//! [`agent::port()`](super::agent::port) reads, the one `PORT` rule
//! for both kinds of entrypoint — and the five paths here are the
//! five exchanges of [`shared::mcp`](crate::shared::mcp) made INTO
//! it, each forwarded by the proxy as one call to that server. The
//! proxy holds one MCP client to it, dialled on the first path opened
//! and kept for the container's life, re-dialled if it died; nothing
//! else of the server's is kept.
//!
//! The mirror of the `/mcp/*` answer paths: there the CONTAINER asks
//! and the caller's servers answer, relayed outward; here the CALLER
//! asks — on a [`tools`](crate::endpoints::containers::tools) scope,
//! `McpListTools` and its siblings — and the container's server
//! answers, relayed inward. Same five shapes, opposite way round,
//! which is why every type here is the shared one re-exported.
//!
//! | the proxy's path | it does | answers with |
//! |------------------|---------|--------------|
//! | `/tool/list-tools` | `tools/list`, params the first message | one [`list_tools::response::Frame`], then the close |
//! | `/tool/list-resources` | `resources/list` | one [`list_resources::response::Frame`], then the close |
//! | `/tool/call-tool` | `tools/call` | one [`call_tool::response::Frame`], then the close |
//! | `/tool/read-resource` | `resources/read` | one [`read_resource::response::Frame`], then the close |
//! | `/tool/notifications` | nothing sent; the opening subscribes | a [`notifications::response::Frame`] per notification, for the connection's life |
//!
//! A server that cannot be dialled, or a call the transport lost, is
//! the exchange's `Error` with an internal-error code and the reason
//! as its message; a refusal the server itself sent travels through
//! as itself, code and all. A first message that is not binary, or
//! params that will not parse, is the `Error` too — invalid params —
//! never a silent close: the caller asked something and is owed an
//! answer in the vocabulary it asked in.
//!
//! The provider's server opens these only on a tool container, the
//! way it opens `/agent/*` only on an agent container: the proxy
//! cannot tell the kinds apart and does not try.

pub mod call_tool;
pub mod list_resources;
pub mod list_tools;
pub mod notifications;
pub mod read_resource;
