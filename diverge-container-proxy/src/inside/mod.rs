//! The program's side: what the program beside the proxy dials, on
//! the loopback, and what each becomes on the begin scope.
//!
//! The daemon protocol as a WebSocket at
//! `/daemon` for a tool container, a compliant MCP server at `/mcp`
//! for an agent container, and a pgwire listener that is a database
//! to the program's driver. None of them changes with the wire
//! outside: each is the proxy's own ask, a channel on the begin
//! scope, and its answer handed back untouched; the two a family does
//! not have answer the program that it has none.

pub mod daemon;
pub mod pairs;
pub mod mcp;
pub mod postgres;
