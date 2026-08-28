//! What an enqueue asks.

use serde::{Deserialize, Serialize};

/// A message for the running conversation's queue.
///
/// The same statement the channel-level
/// [`Enqueue`](crate::endpoints::agentic_loop::run::client::channel_request::Enqueue)
/// makes, at the container's door: plain text, queued and never
/// interrupting — the turn in flight always runs to completion, and
/// the agent takes the message at a seam of its own choosing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// The message's text.
    pub prompt: String,
}
