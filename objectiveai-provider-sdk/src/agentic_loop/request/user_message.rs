//! A message from the caller.

use rmcp::model::{ContentBlock, MetaObject};
use serde::{Deserialize, Serialize};

/// Input from whoever is driving the loop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    /// The discriminator. See [`Message`](super::Message).
    pub role: UserRole,
    /// The content. A LIST of blocks, not one, so text and an image
    /// arrive as a single message rather than as two that a provider
    /// then has to guess were meant together.
    pub content: Vec<ContentBlock>,
    /// Arbitrary protocol-level metadata, MCP's `_meta` extension bag.
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// [`UserMessage`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserRole {
    #[serde(rename = "user")]
    #[default]
    User,
}
