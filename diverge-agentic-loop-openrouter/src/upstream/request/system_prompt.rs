//! The agent's system prompt.

use serde::{Deserialize, Serialize};

/// The role of a [`SystemPrompt`].
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SystemPromptRole {
    System,
    Developer,
}

/// An agent's system prompt — a role and its text content.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
)]
pub struct SystemPrompt {
    /// Whether this is a system or developer message.
    pub role: SystemPromptRole,
    /// The prompt's text content.
    pub content: String,
}
