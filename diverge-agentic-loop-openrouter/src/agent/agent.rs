//! The OpenRouter agent.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ContextCompression, Provider, Reasoning, Stop, Verbosity};

/// An agent running against OpenRouter.
///
/// The widest parameter set of any upstream, because OpenRouter fronts
/// many providers and exposes what each of them accepts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
pub struct Agent {
    /// The model to route to, in OpenRouter's `vendor/name` form.
    pub model: String,
    /// The system prompt's text, sent as the conversation's leading
    /// message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// How many alternatives to report per token. Absent means report
    /// no log probabilities at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u64>,
    /// Penalise tokens by how often they have appeared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Per-token additive bias on the logits, keyed by token id.
    ///
    /// An `IndexMap` rather than a `HashMap`: insertion order is
    /// preserved, so the same bias set serializes identically every
    /// time instead of shuffling between runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<IndexMap<String, i64>>,
    /// Cap on generated tokens, counting reasoning tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,
    /// Penalise tokens for having appeared at all, regardless of count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    /// Sequences that end generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    /// Sampling temperature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Nucleus sampling threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Cap on generated tokens, excluding reasoning tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Minimum probability, relative to the most likely token, for a
    /// token to be considered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f64>,
    /// Which backing providers to route to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// Reasoning configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
    /// Multiplicative penalty on tokens already produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    /// Sample only from tokens above a threshold scaled by the most
    /// likely token's probability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_a: Option<f64>,
    /// Sample only from the k most likely tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,
    /// How detailed responses should be.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<Verbosity>,
    /// What to do when the request exceeds the context window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_compression: Option<ContextCompression>,
}
