//! Streaming chat completion chunk for vector completions.

use crate::{chat, error};
use serde::{Deserialize, Serialize};

/// A streaming chat completion chunk from a single LLM within a vector completion.
///
/// The `index` field is used to correlate chunks belonging to the same
/// underlying completion when accumulating via [`push`](Self::push).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChunk {
    /// Index used to correlate chunks from the same completion.
    pub index: u64,
    /// The underlying chat completion chunk.
    #[serde(flatten)]
    pub inner: chat::completions::response::streaming::ChatCompletionChunk,
    /// Error details if this completion failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<error::ResponseError>,
}

impl ChatCompletionChunk {
    pub fn push(&mut self, other: &ChatCompletionChunk) {
        self.inner.push(&other.inner);
        match (&mut self.error, &other.error) {
            (None, Some(other_error)) => {
                self.error = Some(other_error.clone());
            }
            _ => {}
        }
    }
}
