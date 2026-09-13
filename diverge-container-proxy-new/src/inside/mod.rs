//! The program's side: what the program beside the proxy dials, on
//! the loopback, and what each becomes on the begin scope.
//!
//! The vault as plain HTTP, a command as one streamed `POST`, a
//! compliant MCP server at `/mcp`, and a pgwire listener that is a
//! database to the program's driver. None of them changes with the
//! wire outside: each is the proxy's own ask, a channel on the begin
//! scope, and its answer handed back untouched.

pub mod command;
pub mod mcp;
pub mod postgres;
pub mod vault;
