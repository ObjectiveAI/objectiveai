//! The Codex SDK agent.

use serde::{Deserialize, Serialize};

use super::{Effort, OutputMode, Upstream};

/// An agent running against the Codex SDK.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Agent {
    /// The discriminator. Always `codex_sdk`.
    pub upstream: Upstream,
    /// The model to run.
    pub model: String,
    /// How output is constrained.
    pub output_mode: OutputMode,
    /// How much effort to spend reasoning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    /// Whether the model may search the web.
    ///
    /// A capability, not a tool on
    /// [`AgenticLoopRequest::tools`](super::super::super::AgenticLoopRequest::tools):
    /// the SDK performs the search itself rather than calling back out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_search_enabled: Option<bool>,
}
