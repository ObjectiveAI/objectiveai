//! Object type for streaming chat completion responses.

use serde::{Deserialize, Serialize};

/// The object type for streaming chat completion chunks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Object {
    /// A chat completion chunk object.
    #[serde(rename = "chat.completion.chunk")]
    #[default]
    ChatCompletionChunk,
}
