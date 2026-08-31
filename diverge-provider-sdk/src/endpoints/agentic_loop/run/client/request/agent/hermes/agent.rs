//! The Hermes agent.

use serde::{Deserialize, Serialize};

use super::{Effort, Provider, Toolsets, Upstream};

/// An agent running against Hermes (Nous Research's agent harness).
///
/// Every field here is HONORABLE: it names a mechanism the container
/// actually has — a `config.yaml` key its harness writes before
/// Hermes starts, or a per-request field on the run it drives — and
/// nothing else made the cut. What Hermes cannot be told through its
/// wire is not here: no skill preloading (skills arrive as mounts
/// and Hermes indexes them itself), no temperature (Hermes has no
/// main-agent sampling key), no thinking-token budget (the
/// [`effort`](Self::effort) ladder is the whole reasoning
/// vocabulary).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    /// The discriminator. Always `hermes`.
    pub upstream: Upstream,
    /// The inference source, credentials and all — Hermes is
    /// multi-provider by design, this picks which, and the pick
    /// carries its own auth as arguments. See [`Provider`].
    pub provider: Provider,
    /// The model to run, in the provider's own naming.
    pub model: String,
    /// The system prompt, applied per request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Reasoning effort. Absent = the provider's own default;
    /// [`none`](Effort::None) = explicitly disabled — Hermes's own
    /// disable sentinel, which is why absence and `none` are
    /// different facts. See [`Effort`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    /// Hermes's LOCAL capabilities, one switch each: a tri-state
    /// bool where a tool takes nothing, that tool's own argument
    /// structure where it does — absent always tracks Hermes's
    /// default. (The caller's MCP tools ride beside these
    /// regardless; they are not in this vocabulary.) See
    /// [`Toolsets`].
    #[serde(default, skip_serializing_if = "Toolsets::unsaid")]
    pub toolsets: Toolsets,
}
