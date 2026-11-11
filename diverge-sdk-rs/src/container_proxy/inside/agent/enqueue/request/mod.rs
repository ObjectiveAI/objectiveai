//! The body: the messages, joined.
//!
//! The provider protocol's enqueue carries the same: a batch of
//! [`Message`](crate::shared::containers::enqueue::request::Message)s,
//! in order. This is what the proxy makes of the batches it holds:
//! every message of every one, in order, each under its key, the
//! same `{"key": "…", "content": […]}` pairs a
//! [`run`](crate::container_proxy::inside::agent::run) carries.

mod request;

pub use request::*;
