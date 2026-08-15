//! The MCP answer a provider relays out of the container.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. An MCP answer is an HTTP response, and
//! [`http::response`](crate::shared::http::response) already says what one is
//! and why it arrives head-first. What the frame CONTAINS is named
//! through that module, where it is defined.

mod frame;

pub use frame::*;
