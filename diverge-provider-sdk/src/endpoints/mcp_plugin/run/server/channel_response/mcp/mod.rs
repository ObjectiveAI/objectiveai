//! The MCP answer a provider relays out of the container.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. An MCP answer is an HTTP response, and
//! [`http::response`](crate::shared::http::response) already says what one is
//! and why it arrives head-first. What the frame CONTAINS is named
//! through that module, where it is defined.
//!
//! It is the same alias a
//! [`laboratory run`](crate::endpoints::laboratories::run::server::channel_response::mcp)
//! makes, and that is not duplication: each endpoint states what rides
//! its own channels, and two endpoints happening to relay the same
//! thing is a fact about them rather than a shared abstraction one of
//! them should own.

mod frame;

pub use frame::*;
