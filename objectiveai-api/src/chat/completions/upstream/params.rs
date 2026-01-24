//! Request parameters for upstream provider calls.

use std::sync::Arc;

/// Parameters for an upstream completion request.
#[derive(Debug, Clone)]
pub enum Params {
    /// Parameters for a chat completion request.
    Chat {
        /// The chat completion request.
        request: Arc<objectiveai::chat::completions::request::ChatCompletionCreateParams>,
    },
    /// Parameters for a chat completion used to collect an LLM vote for vector completions.
    Vector {
        /// The vector completion request.
        request: Arc<objectiveai::vector::completions::request::VectorCompletionCreateParams>,
        /// Maps prefix keys shown to the LLM (e.g., "`A`") to response indices in the original request.
        vector_pfx_indices: Arc<Vec<(String, usize)>>,
    },
}
