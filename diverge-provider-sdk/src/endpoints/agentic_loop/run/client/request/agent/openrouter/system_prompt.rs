//! The agent's system prompt.

use serde::{Deserialize, Serialize};

/// The agent's system prompt — a role and its text content.
///
/// Sent as the conversation's leading message, under whichever of the
/// two instruction roles the model expects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPrompt {
    /// Whether this is a system or developer message.
    pub role: SystemPromptRole,
    /// The prompt's text content.
    pub content: String,
}

/// The role of a [`SystemPrompt`].
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SystemPromptRole {
    /// The `system` role.
    #[default]
    System,
    /// The `developer` role, which newer OpenAI-family models expect
    /// in place of `system`.
    Developer,
}
