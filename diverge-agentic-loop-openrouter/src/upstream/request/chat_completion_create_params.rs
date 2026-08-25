//! Chat completion request parameters for OpenRouter.

use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::openrouter;
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

/// The agent's context compression, as the plugin entry it rides in
/// as: the id is the plugin's, and the engine is the enum's own wire
/// string.
impl From<openrouter::ContextCompression> for Plugin {
    fn from(compression: openrouter::ContextCompression) -> Self {
        let engine = serde_json::to_value(compression)
            .ok()
            .and_then(|value| value.as_str().map(String::from));
        Plugin {
            id: "context-compression".to_string(),
            engine,
        }
    }
}

impl ChatCompletionCreateParams {
    /// Build an OpenRouter request from the provider request's fields
    /// — the agent, already known to be the `openrouter` kind, and
    /// the prompt. The continuation is not taken: resumption is this
    /// container's own state, applied to the messages elsewhere.
    ///
    /// Every parameter the agent carries moves across, delegating to
    /// the sub-type conversions beside each type. `tools` is `None`
    /// here — tools are not a request field; they come from the MCP
    /// proxy and are the loop's to add.
    pub fn new(
        agent: openrouter::Agent,
        prompt: Vec<rmcp::model::ContentBlock>,
    ) -> Self {
        // Log probabilities are reported only when the agent asked
        // for a positive count; zero is the same statement as absent.
        let top_logprobs = agent.top_logprobs.filter(|count| *count > 0);
        Self {
            messages: messages(prompt),
            provider: agent.provider.map(Into::into),
            model: agent.model,
            frequency_penalty: agent.frequency_penalty,
            logit_bias: agent.logit_bias,
            max_completion_tokens: agent.max_completion_tokens,
            presence_penalty: agent.presence_penalty,
            stop: agent.stop.map(Into::into),
            temperature: agent.temperature,
            top_p: agent.top_p,
            max_tokens: agent.max_tokens,
            min_p: agent.min_p,
            reasoning: agent.reasoning.map(Into::into),
            repetition_penalty: agent.repetition_penalty,
            top_a: agent.top_a,
            top_k: agent.top_k,
            verbosity: agent.verbosity.map(Into::into),
            plugins: agent
                .context_compression
                .map(|compression| vec![compression.into()]),
            logprobs: top_logprobs.map(|_| true),
            top_logprobs,
            tools: None,
            stream: true,
            stream_options: super::StreamOptions {
                include_usage: Some(true),
            },
            usage: super::Usage { include: true },
        }
    }
}

/// The message array: the prompt (and, on resumption, the state the
/// continuation names) as OpenRouter messages.
fn messages(prompt: Vec<rmcp::model::ContentBlock>) -> Vec<super::Message> {
    let _ = prompt;
    unimplemented!("the message array is built with its own care")
}
