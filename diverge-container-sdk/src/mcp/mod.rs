//! The proxy's MCP server, as a client.
//!
//! At `/mcp/agent` the proxy is a compliant MCP server whose answers
//! all live with the caller: what it lists and what it calls are the
//! caller's servers, relayed. [`Client`] dials it and initializes,
//! and the four exchanges are its four methods. Notifications are
//! rmcp's business — the [`ClientHandler`](rmcp::ClientHandler) a
//! client is started with receives them — so a program that wants
//! them passes a handler, and one that does not passes nothing.

mod client;
mod error;

pub use client::*;
pub use error::*;
