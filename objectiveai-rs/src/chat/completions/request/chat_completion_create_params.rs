//! Chat completion request parameters.

use serde::{Deserialize, Serialize};

/// Parameters for creating a chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionCreateParams {
    /// The conversation messages.
    pub messages: Vec<super::Message>,
    /// Provider routing preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<super::Provider>,
    /// The model to use (inline Ensemble LLM or stored ID).
    pub model: super::Model,
    /// Alternative models to try if the primary model fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<super::Model>>,
    /// Number of top log probabilities to return per token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u64>,
    /// Output format constraints (text, JSON, or JSON schema).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<super::ResponseFormat>,
    /// Random seed for deterministic generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// How the model should use tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<super::ToolChoice>,
    /// Available tools/functions the model can call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<super::Tool>>,
    /// Whether the model can make multiple tool calls in parallel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Predicted output for speculative decoding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<super::Prediction>,

    // --- Retry configuration ---

    /// Maximum elapsed time (ms) for exponential backoff retries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_max_elapsed_time: Option<u64>,
    /// Timeout (ms) for receiving the first chunk of a streaming response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_chunk_timeout: Option<u64>,
    /// Timeout (ms) between subsequent chunks of a streaming response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_chunk_timeout: Option<u64>,
}
