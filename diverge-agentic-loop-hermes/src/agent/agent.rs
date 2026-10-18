//! The Hermes agent.

use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::secrets::Declaration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Effort, Provider, Toolsets};
use crate::filesystem::{self, PrepareError};

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
///
/// # Secrets
///
/// No credential is a field. The provider and the toolsets SELECT
/// secrets by name — each structure's `SECRET:` paragraph says which,
/// and in what mode — and the agent's template must declare every
/// name selected. [`declaration`](Self::declaration) is that set, as
/// the registration answers it; the proxy feeds the values before
/// every loop, and collects the rotating ones after.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Agent {
    /// The inference source — Hermes is multi-provider by design,
    /// this picks which, and the pick selects the secrets its auth
    /// needs. See [`Provider`].
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
    /// `Option<bool>` where a tool takes nothing but at most a
    /// secret, that tool's own argument structure where it takes
    /// more — absent is off either way, and the whole structure
    /// absent is everything off. (The caller's MCP tools ride beside
    /// these regardless, and the tool containers the agent asks for
    /// are [`dependencies`](Self::dependencies).) See [`Toolsets`].
    #[serde(default)]
    pub toolsets: Toolsets,
    /// The tool containers this agent depends on, each a dependency
    /// tool template the caller deploys and serves to it as an MCP
    /// server, exactly as the SDK's [`Template`] states one: that
    /// type's JSON Schema is what `--schema` prints for this member,
    /// in `/DIVERGE.md`, and this list, unchanged, is what the
    /// registration answers — one schema, from the one type, at both.
    /// Absent is none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Template>,
}

impl Agent {
    /// The secrets this agent takes, each by name to its mode: what
    /// the registration answers. Computed by planning the agent
    /// ([`filesystem::plan`]) — which probes the skills mount, hence
    /// async — so the names are exactly the ones the run will apply;
    /// an agent that cannot be planned (a contradiction, a mount
    /// that cannot be looked at) is the registration's refusal.
    pub async fn declaration(&self) -> Result<Declaration, PrepareError> {
        Ok(filesystem::plan(self).await?.declaration())
    }
}
