//! The Hermes agent.

use serde::{Deserialize, Serialize};

use super::{Effort, Provider, Toolset, Upstream};

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
    /// The inference source — Hermes is multi-provider by design,
    /// and this picks which. See [`Provider`].
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
    /// A custom OpenAI-compatible endpoint, for the providers that
    /// take one ([`custom`](Provider::Custom),
    /// [`azure-foundry`](Provider::AzureFoundry), …) — including
    /// anything local Hermes aliases onto `custom`: ollama, vllm,
    /// llama.cpp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Hermes's LOCAL capabilities. Absent = Hermes's defaults;
    /// present = exactly these — and empty means none, an agent
    /// whose only tools are the caller's own, arriving over MCP.
    /// (The caller's MCP tools ride beside this selection
    /// regardless; they are not part of this vocabulary.) See
    /// [`Toolset`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolsets: Option<Vec<Toolset>>,
    /// The turn cap; absent = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u64>,
    /// The wall-clock budget, in seconds; absent = none. Hermes
    /// itself injects a wrap-up notice into the conversation at 80%
    /// of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_budget_seconds: Option<f64>,
}
