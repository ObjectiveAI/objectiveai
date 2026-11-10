//! Object type for streaming responses.

use serde::Deserialize;

/// The object type for streaming chat completion chunks.
#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub enum Object {
    /// A chat completion chunk object.
    #[serde(rename = "chat.completion.chunk")]
    #[default]
    ChatCompletionChunk,
}
