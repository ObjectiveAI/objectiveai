//! A tool container, from the proxy's side.
//!
//! The image runs an MCP server, and the server reaches it through
//! the proxy: the arguments ride the begin, an info answers the
//! image's DIVERGE.md, which says what they may be, and the five
//! exchanges [`shared::mcp`](crate::shared::mcp)
//! defines ride a channel each — the same exchanges a caller opens on
//! the provider, carried the last hop. [`begin`] is
//! the one scope: it opens the connection's work, and every one of
//! those is a channel on it.

pub mod begin;
