//! The registry answers a caller sends back.
//!
//! What rides this channel is the OCI Distribution Specification —
//! the pull half of it, exactly as a
//! [`laboratory run`](crate::endpoints::laboratories::run::client::channel_response::oci)
//! uses it, and for the same reasons. A provider's container runtime
//! issues ordinary registry requests, the provider relays them, and a
//! caller's own registry answers. Nothing on either side is aware of
//! the other.
//!
//! An image is an image. Nothing about serving one changes because the
//! container built from it will answer tool calls rather than host an
//! agent, which is why this module has nothing of its own to say.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. It is an alias because a registry
//! answer is an HTTP response, and
//! [`http::response`](crate::shared::http::response) already says what one is
//! and why it arrives head-first. What the frame CONTAINS is named
//! through that module, where it is defined.

mod frame;

pub use frame::*;
