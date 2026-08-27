//! The Claude Code agent.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{Effort, Upstream};

/// An agent running against Claude Code.
///
/// Far fewer knobs than OpenRouter's, and not because anything is
/// missing: Claude Code is an agent harness in its own right, so
/// the sampling decisions OpenRouter exposes are made inside it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Agent {
    /// The discriminator. Always `claude_code`.
    pub upstream: Upstream,
    /// The model to run.
    pub model: String,
    /// Whether to think before answering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<bool>,
    /// How much effort to spend thinking. Meaningful only with
    /// [`thinking`](Self::thinking) on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    /// The skills the agent runs with, keyed by skill name — the
    /// directory name Claude Code treats as the skill's identity.
    /// Each value is the skill directory's dirhash: the deterministic
    /// content identity of its files, by which the provider knows
    /// WHICH skill without carrying the skill itself in the request.
    ///
    /// An `IndexMap` rather than a `HashMap`: insertion order is
    /// preserved, so the same skill set serializes identically every
    /// time instead of shuffling between runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<IndexMap<String, String>>,
    /// The subagents the agent runs with, keyed by agent name. Each
    /// value is the agent definition's dirhash — the same
    /// deterministic content identity [`skills`](Self::skills) uses.
    ///
    /// Named for the definition FORMAT, not the field's address:
    /// skills share one client-side folder because their format is
    /// harness-agnostic, but agent definitions do not — Claude Code's
    /// markdown-with-frontmatter files and Codex's TOML cannot share
    /// a directory, so the folder and the field carry the format's
    /// name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_code_agents: Option<IndexMap<String, String>>,
}
