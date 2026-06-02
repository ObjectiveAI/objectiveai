//! Agent completion request parameters.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for creating a agent completion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.request.AgentCompletionCreateParams")]
pub struct AgentCompletionCreateParams {
    /// The conversation messages.
    pub messages: Vec<super::super::message::Message>,
    /// Provider routing preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<super::Provider>,
    /// The agent to use (inline Agent or stored ID).
    pub agent: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    /// Output format constraints (text, JSON, or JSON schema).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_format: Option<super::ResponseFormatParam>,
    /// Random seed for deterministic generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    /// Continuation from a previous completion, as a base64-encoded string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<String>,
}

