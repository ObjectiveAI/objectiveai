//! User messages.

use super::super::RichContent;
use serde::Serialize;

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
    /// One turn's prompt, as its text.
    pub fn new(prompt: String) -> Self {
        UserMessage {
            content: RichContent::Text(prompt),
        }
    }
}
