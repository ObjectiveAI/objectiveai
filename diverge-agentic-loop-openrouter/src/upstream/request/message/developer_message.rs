//! Developer messages.

use serde::{Deserialize, Serialize};

/// A developer message — the agent's system prompt under the
/// `developer` role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeveloperMessage {
    /// The prompt's text content.
    pub content: String,
}
