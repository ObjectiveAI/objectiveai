//! The Claude Code agent.

use serde::{Deserialize, Serialize};

use super::{Effort, Tools, Upstream};

/// An agent running against Claude Code.
///
/// Far fewer knobs than OpenRouter's, and not because anything is
/// missing: Claude Code is an agent harness in its own right, so
/// the sampling decisions OpenRouter exposes are made inside it.
/// What it does expose is its tools: [`tools`](Self::tools) is the
/// exact set of built-ins the model is given, every switch stated,
/// passed to Claude Code as `--tools`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Agent {
    /// The discriminator. Always `claude_code`.
    pub upstream: Upstream,
    /// The model to run.
    pub model: String,
    /// The switchable built-in tools, every one stated. The MCP
    /// tools and `Skill` are on regardless; see [`Tools`].
    pub tools: Tools,
    /// Whether to think before answering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<bool>,
    /// How much effort to spend thinking. Meaningful only with
    /// [`thinking`](Self::thinking) on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
}
