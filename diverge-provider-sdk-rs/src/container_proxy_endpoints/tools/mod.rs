//! A tool container, from the proxy's side.
//!
//! The image runs an MCP server, and the server reaches it through
//! the proxy over the five exchanges [`shared::mcp`](crate::shared::mcp)
//! defines, each on a channel of its own — the same exchanges a
//! caller opens on the provider, carried the last hop. [`begin`] is
//! the one scope: it opens the connection's work, and every one of
//! those is a channel on it.

pub mod begin;
