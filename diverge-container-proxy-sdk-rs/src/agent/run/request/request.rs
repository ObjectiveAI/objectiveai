//! What one loop is asked.

use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};

/// Run one loop on this message.
///
/// The arguments are not here: they were registered once, and they
/// never change. The message is each loop's, because a container
/// runs loops one after another — each resuming the conversation the
/// last one left — and every one is asked something: a list of MCP
/// content blocks, as an enqueue carries one. When several messages
/// waited for the loop, the content is all of theirs, concatenated
/// in the order they were enqueued. A program that cannot take a
/// block answers a `4xx` — the whole message refused, in its own
/// words — and the proxy answers every message the loop was to take
/// with them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// What the loop is asked, in order.
    pub content: Vec<ContentBlock>,
}
