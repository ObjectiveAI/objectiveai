//! User messages.

use rmcp::model::ContentBlock;
use serde::Serialize;

use super::super::RichContent;

/// A user message from the end user.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
)]
pub struct UserMessage {
    /// The message content (supports text, images, audio, video, files).
    pub content: RichContent,
}

impl UserMessage {
    /// One turn's message, its MCP content blocks as OpenRouter's
    /// parts — the same conversion a tool result's content gets, so
    /// what a caller says and what a tool answers cross the same
    /// bridge.
    pub fn new(content: Vec<ContentBlock>) -> Self {
        UserMessage {
            content: RichContent::Parts(content.into_iter().map(Into::into).collect()),
        }
    }
}
