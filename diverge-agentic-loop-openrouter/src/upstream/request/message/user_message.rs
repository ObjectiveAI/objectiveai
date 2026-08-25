//! User messages.

use super::super::RichContent;
use serde::{Deserialize, Serialize};

/// A user message from the end user.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub struct UserMessage {
    /// The message content (supports text, images, audio, video, files).
    pub content: RichContent,
}
