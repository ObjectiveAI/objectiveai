//! Vector completion request parameters.

use crate::agent;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

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

// Placeholder `ProducesRequestFiles` impl: dumps the whole params as one
// summary JSON without extracting any leaves. Lets the
// [`crate::filesystem::logs::LogWriter`]'s deferred-request pipeline
// stay homogeneous across factories while this type still uses the
// monolithic on-disk shape. Phase 2 will swap this for an actual
// per-leaf extraction (see `agent_completion_create_params.rs` for the
// reference pattern).
#[cfg(feature = "filesystem")]
impl crate::filesystem::logs::ProducesRequestFiles for VectorCompletionCreateParams {
    fn produce_files(
        &self,
        id: &str,
        route_base: &str,
    ) -> (
        crate::filesystem::logs::LogReference,
        Vec<crate::filesystem::logs::LogFile>,
    ) {
        use crate::filesystem::logs::{LogFile, LogReference};
        let summary = LogFile {
            route: route_base.to_string(),
            id: id.to_string(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(self)
                .expect("VectorCompletionCreateParams serializes"),
        };
        let reference = LogReference::new(summary.path());
        (reference, vec![summary])
    }
}
