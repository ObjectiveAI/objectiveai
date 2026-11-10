//! Developer messages.

use serde::Serialize;

/// A developer message — the agent's system prompt under the
/// `developer` role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeveloperMessage {
    /// The prompt's text content.
    pub content: String,
}
