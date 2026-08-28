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
    /// One turn's prompt, each content block as the part it is.
    pub fn new(prompt: Vec<rmcp::model::ContentBlock>) -> Self {
        UserMessage {
            content: RichContent::Parts(
                prompt.into_iter().map(Into::into).collect(),
            ),
        }
    }
}
