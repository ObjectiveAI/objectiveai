//! Vector completion request parameters.

use crate::agent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for creating a vector completion.
///
/// Vector completions run multiple agent completions (one per LLM in the
/// swarm), force each to vote for one of the predefined responses, and
/// combine votes using the provided profile weights to produce final scores.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "vector.completions.request.VectorCompletionCreateParams")]
pub struct VectorCompletionCreateParams {
    // --- Caching and retry options ---
    /// If present, reuses votes from a previous request with this ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub retry: Option<String>,
    /// If true, uses cached votes when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub from_cache: Option<bool>,

    // --- Core configuration ---
    /// The conversation messages (the prompt).
    pub messages: Vec<agent::completions::message::Message>,
    /// Provider routing preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<agent::completions::request::Provider>,
    /// The Swarm of agents to use.
    pub swarm: crate::swarm::InlineSwarmBaseOrRemoteCommitOptional,
    /// Random seed for deterministic results.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    /// The possible responses the LLMs can vote for.
    pub responses: Vec<agent::completions::message::RichContent>,
    /// Continuation from a previous completion, as a base64-encoded string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<String>,
}

