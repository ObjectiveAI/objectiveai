//! The Hermes agent.

use diverge_provider_sdk::shared::containers::tools::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Effort, Provider, Toolsets};

/// An agent running against Hermes (Nous Research's agent harness).
///
/// Every field here is HONORABLE: it names a mechanism the container
/// actually has — a `config.yaml` key its harness writes before
/// Hermes starts, or a per-request field on the run it drives — and
/// nothing else made the cut. What Hermes cannot be told through its
/// wire is not here: no skill preloading (skills arrive as mounts
/// under `/root/.hermes/external-skills/`, one directory per skill
/// holding its `SKILL.md`, and Hermes indexes them itself — that
/// path, not its own `skills/`, which it writes into), no
/// temperature (Hermes has no
/// main-agent sampling key), no thinking-token budget (the
/// [`effort`](Self::effort) ladder is the whole reasoning
/// vocabulary).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Agent {
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
    /// Hermes's LOCAL capabilities, one switch each: an
    /// `Option<bool>` where a tool takes nothing, that tool's own
    /// argument structure where it does — absent is off either way,
    /// and the whole structure absent is everything off. (The
    /// caller's MCP tools ride beside these regardless, and the tool
    /// containers the agent asks for are
    /// [`mcp_tools`](Self::mcp_tools).) See [`Toolsets`].
    #[serde(default)]
    pub toolsets: Toolsets,
    /// The tool containers this agent depends on, each one the caller
    /// runs and serves to it as an MCP server, in the form the
    /// provider's wire defines. Passed back whole as the registration's
    /// answer, which is how the caller learns of them. Absent is none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_tools: Vec<Tool>,
}
