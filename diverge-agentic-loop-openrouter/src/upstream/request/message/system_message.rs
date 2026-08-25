//! System messages.

use serde::{Deserialize, Serialize};

/// A system message — the agent's system prompt under the `system`
/// role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemMessage {
    /// The prompt's text content.
    pub content: String,
}
