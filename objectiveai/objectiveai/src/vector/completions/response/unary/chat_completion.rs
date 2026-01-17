//! Chat completion wrapper for vector completions.

use crate::{chat, error, vector::completions::response};
use serde::{Deserialize, Serialize};

/// A chat completion from a single LLM within a vector completion.
///
/// Wraps the standard chat completion response with an index to identify
/// which LLM in the ensemble produced it, and an optional error if the
/// completion failed.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletion {
    /// Index of this completion within the vector completion.
    pub index: u64,
    /// The underlying chat completion response.
    #[serde(flatten)]
    pub inner: chat::completions::response::unary::ChatCompletion,
    /// Error details if this completion failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<error::ResponseError>,
}

impl From<response::streaming::ChatCompletionChunk> for ChatCompletion {
    fn from(
        response::streaming::ChatCompletionChunk {
            index,
            inner,
            error,
        }: response::streaming::ChatCompletionChunk,
    ) -> Self {
        Self {
            index,
            inner: chat::completions::response::unary::ChatCompletion::from(
                inner,
            ),
            error,
        }
    }
}
