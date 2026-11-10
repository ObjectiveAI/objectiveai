//! One message of a batch.

use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};

/// One message: the key and the content. An enqueue's
/// [`Request`](super::Request) carries these, in order, and so do the
/// two calls the proxy makes of them inside the container — a
/// [`run`](crate::container_proxy::inside::agent::run) and an
/// [`enqueue`](crate::container_proxy::inside::agent::enqueue) — so
/// the message is one type from the caller to the agent's program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The caller's handle on the message: what each of its user
    /// parts carries back, so the log credits its sender. Not unique —
    /// two messages may share one — and not read by the provider or
    /// the proxy, which carry it and nothing else.
    pub key: String,
    /// The message's content, in order.
    pub content: Vec<ContentBlock>,
}
