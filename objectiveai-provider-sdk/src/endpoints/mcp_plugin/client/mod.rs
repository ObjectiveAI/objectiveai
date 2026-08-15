//! The client side of an MCP plugin: what a client sends.
//!
//! [`request`] opens the scope. Nothing else yet — the channels that
//! reach into a running plugin are not written.

pub mod request;
