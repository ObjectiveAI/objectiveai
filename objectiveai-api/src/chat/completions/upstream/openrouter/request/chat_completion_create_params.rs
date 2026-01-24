//! Chat completion request parameters for OpenRouter.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Chat completion request parameters formatted for the OpenRouter API.
///
/// Combines parameters from both the Ensemble LLM configuration and the
/// incoming request to create a complete request for OpenRouter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionCreateParams {
    /// Messages for the conversation, including any prefix/suffix from the Ensemble LLM.
    pub messages: Vec<objectiveai::chat::completions::request::Message>,
    /// Provider preferences merged from request and Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<super::Provider>,

    /// The model identifier from the Ensemble LLM.
    pub model: String,
    /// Frequency penalty from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Logit bias from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<IndexMap<String, i64>>,
    /// Maximum completion tokens from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,
    /// Presence penalty from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    /// Stop sequences from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<objectiveai::ensemble_llm::Stop>,
    /// Temperature from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Top-p (nucleus sampling) from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Maximum tokens (legacy) from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Min-p sampling from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f64>,
    /// Reasoning configuration from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<objectiveai::ensemble_llm::Reasoning>,
    /// Repetition penalty from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    /// Top-a sampling from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_a: Option<f64>,
    /// Top-k sampling from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,
    /// Verbosity setting from Ensemble LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<objectiveai::ensemble_llm::Verbosity>,

    /// Whether to include log probabilities from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    /// Number of top log probabilities to return from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u64>,
    /// Response format specification from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<objectiveai::chat::completions::request::ResponseFormat>,
    /// Random seed from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Tool choice configuration from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<objectiveai::chat::completions::request::ToolChoice>,
    /// Available tools from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<objectiveai::chat::completions::request::Tool>>,
    /// Whether to allow parallel tool calls from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Prediction hints from request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<objectiveai::chat::completions::request::Prediction>,

    /// Always true for streaming requests.
    pub stream: bool,
    /// Stream options for usage inclusion.
    pub stream_options: super::StreamOptions,
    /// Usage reporting options.
    pub usage: super::Usage,
}

impl ChatCompletionCreateParams {
    /// Creates request parameters for a chat completion.
    ///
    /// Applies the Ensemble LLM's prefix/suffix messages and decoding parameters.
    pub fn new_for_chat(
        ensemble_llm: &objectiveai::ensemble_llm::EnsembleLlm,
        request: &objectiveai::chat::completions::request::ChatCompletionCreateParams,
    ) -> Self {
        Self {
            messages: super::prompt::new_for_chat(
                ensemble_llm.base.prefix_messages.as_deref(),
                &request.messages,
                ensemble_llm.base.suffix_messages.as_deref(),
            ),
            provider: super::provider::Provider::new(
                request.provider,
                ensemble_llm.base.provider.as_ref(),
            ),
            model: ensemble_llm.base.model.clone(),
            frequency_penalty: ensemble_llm.base.frequency_penalty,
            logit_bias: ensemble_llm.base.logit_bias.clone(),
            max_completion_tokens: ensemble_llm.base.max_completion_tokens,
            presence_penalty: ensemble_llm.base.presence_penalty,
            stop: ensemble_llm.base.stop.clone(),
            temperature: ensemble_llm.base.temperature,
            top_p: ensemble_llm.base.top_p,
            max_tokens: ensemble_llm.base.max_tokens,
            min_p: ensemble_llm.base.min_p,
            reasoning: ensemble_llm.base.reasoning,
            repetition_penalty: ensemble_llm.base.repetition_penalty,
            top_a: ensemble_llm.base.top_a,
            top_k: ensemble_llm.base.top_k,
            verbosity: ensemble_llm.base.verbosity,
            logprobs: if let Some(top_logprobs) = request.top_logprobs {
                Some(top_logprobs > 0)
            } else {
                None
            },
            top_logprobs: request.top_logprobs,
            response_format: request.response_format.clone(),
            seed: request.seed,
            tool_choice: request.tool_choice.clone(),
            tools: request.tools.clone(),
            parallel_tool_calls: request.parallel_tool_calls,
            prediction: request.prediction.clone(),
            stream: true,
            stream_options: super::StreamOptions {
                include_usage: Some(true),
            },
            usage: super::Usage { include: true },
        }
    }

    /// Creates request parameters for a vector completion vote.
    ///
    /// Transforms the vector completion request into a chat completion that asks
    /// the LLM to select from labeled response options.
    pub fn new_for_vector(
        vector_pfx_indices: &[(String, usize)],
        ensemble_llm: &objectiveai::ensemble_llm::EnsembleLlm,
        request: &objectiveai::vector::completions::request::VectorCompletionCreateParams,
    ) -> Self {
        Self {
            messages: super::prompt::new_for_vector(
                &request.responses,
                vector_pfx_indices,
                ensemble_llm.base.output_mode,
                ensemble_llm.base.prefix_messages.as_deref(),
                &request.messages,
                ensemble_llm.base.suffix_messages.as_deref(),
            ),
            provider: super::provider::Provider::new(
                request.provider,
                ensemble_llm.base.provider.as_ref(),
            ),
            model: ensemble_llm.base.model.clone(),
            frequency_penalty: ensemble_llm.base.frequency_penalty,
            logit_bias: ensemble_llm.base.logit_bias.clone(),
            max_completion_tokens: ensemble_llm.base.max_completion_tokens,
            presence_penalty: ensemble_llm.base.presence_penalty,
            stop: ensemble_llm.base.stop.clone(),
            temperature: ensemble_llm.base.temperature,
            top_p: ensemble_llm.base.top_p,
            max_tokens: ensemble_llm.base.max_tokens,
            min_p: ensemble_llm.base.min_p,
            reasoning: ensemble_llm.base.reasoning,
            repetition_penalty: ensemble_llm.base.repetition_penalty,
            top_a: ensemble_llm.base.top_a,
            top_k: ensemble_llm.base.top_k,
            verbosity: ensemble_llm.base.verbosity,
            logprobs: if let Some(top_logprobs) = ensemble_llm.base.top_logprobs {
                Some(top_logprobs > 0)
            } else {
                None
            },
            top_logprobs: ensemble_llm.base.top_logprobs,
            response_format: super::response_format::new_for_vector(
                vector_pfx_indices,
                ensemble_llm.base.output_mode,
                ensemble_llm.base.synthetic_reasoning,
            ),
            seed: request.seed,
            tool_choice: super::tool_choice::new_for_vector(
                ensemble_llm.base.output_mode,
                request.tools.as_deref(),
            ),
            tools: super::tools::new_for_vector(
                vector_pfx_indices,
                ensemble_llm.base.output_mode,
                ensemble_llm.base.synthetic_reasoning,
                request.tools.as_deref(),
            ),
            parallel_tool_calls: None,
            prediction: None,
            stream: true,
            stream_options: super::StreamOptions {
                include_usage: Some(true),
            },
            usage: super::Usage { include: true },
        }
    }
}
