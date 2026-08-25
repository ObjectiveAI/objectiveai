//! Chat completion request parameters for OpenRouter.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// One entry in OpenRouter's request-body `plugins` array. Today the
/// only producer is `context-compression` (see the agent's
/// `context_compression` field), but the shape is OpenRouter-defined
/// — any future plugin id slots in here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
}

/// Chat completion request parameters formatted for the OpenRouter API.
///
/// Combines parameters from both the Agent configuration and the
/// incoming request to create a complete request for OpenRouter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionCreateParams {
    /// Messages for the conversation: the agent's system prompt (if any) as the
    /// leading entry, followed by the conversation (including any prefix/suffix
    /// from the Agent).
    pub messages: Vec<super::Message>,
    /// Provider preferences merged from request and Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<super::Provider>,

    /// The model identifier from the Agent.
    pub model: String,
    /// Frequency penalty from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Logit bias from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<IndexMap<String, i64>>,
    /// Maximum completion tokens from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,
    /// Presence penalty from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    /// Stop sequences from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<super::Stop>,
    /// Temperature from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Top-p (nucleus sampling) from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Maximum tokens (legacy) from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Min-p sampling from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f64>,
    /// Reasoning configuration from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<super::Reasoning>,
    /// Repetition penalty from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    /// Top-a sampling from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_a: Option<f64>,
    /// Top-k sampling from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,
    /// Verbosity setting from Agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<super::Verbosity>,
    /// Plugins array derived from Agent (currently only sourced from
    /// `context_compression`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<Plugin>>,

    /// Whether to include log probabilities from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    /// Number of top log probabilities to return from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u64>,
    /// Available tools (MCP + response format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<super::Tool>>,

    /// Always true for streaming requests.
    pub stream: bool,
    /// Stream options for usage inclusion.
    pub stream_options: super::StreamOptions,
    /// Usage reporting options.
    pub usage: super::Usage,
}
