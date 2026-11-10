//! The body: the messages, joined.
//!
//! Not the provider protocol's enqueue — that names one message under
//! one key, and stays the provider wire's. This is the proxy's batch:
//! every message it holds, in enqueue order, each under its key, the
//! same `{"key": "…", "content": […]}` pairs a
//! [`run`](crate::container_proxy::inside::agent::run) carries.

mod request;

pub use request::*;
