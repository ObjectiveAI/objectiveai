//! The Codex agent.

use serde::{Deserialize, Serialize};

use super::{Effort, Upstream};

/// An agent running against Codex.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Agent {
    /// The discriminator. Always `codex`.
    pub upstream: Upstream,
    /// The model to run.
    pub model: String,
    /// How much effort to spend reasoning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    /// Whether the model may search the web.
    ///
    /// A capability of the upstream rather than a tool: Codex
    /// performs the search itself rather than calling back out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_search_enabled: Option<bool>,
}
