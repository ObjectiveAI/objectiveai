//! System messages.

use serde::Serialize;

/// A system message — the agent's system prompt under the `system`
/// role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SystemMessage {
    /// The prompt's text content.
    pub content: String,
}

impl SystemMessage {
    /// The agent's system prompt, as the message it leads with.
    pub fn new(content: String) -> Self {
        SystemMessage { content }
    }
}
