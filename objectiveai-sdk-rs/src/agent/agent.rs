//! Core Agent types — unified enum dispatching to per-upstream implementations.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Pre-validation types (no computed ID) ──────────────────────────

/// The base inline configuration for an Agent (without computed ID or metadata).
///
/// This is an untagged enum that dispatches to the per-upstream AgentBase.
/// Deserialization tries each variant in order until one matches.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(untagged)]
#[schemars(rename = "agent.InlineAgentBase")]
pub enum InlineAgentBase {
    #[schemars(title = "Openrouter")]
    Openrouter(super::openrouter::AgentBase),
    #[schemars(title = "ClaudeAgentSdk")]
    ClaudeAgentSdk(super::claude_agent_sdk::AgentBase),
    #[schemars(title = "CodexSdk")]
    CodexSdk(super::codex_sdk::AgentBase),
    #[schemars(title = "Mock")]
    Mock(super::mock::AgentBase),
}

impl InlineAgentBase {
    pub fn as_ref(&self) -> InlineAgentRef<'_> {
        match self {
            InlineAgentBase::Openrouter(b) => InlineAgentRef::Openrouter(b),
            InlineAgentBase::ClaudeAgentSdk(b) => {
                InlineAgentRef::ClaudeAgentSdk(b)
            }
            InlineAgentBase::CodexSdk(b) => InlineAgentRef::CodexSdk(b),
            InlineAgentBase::Mock(b) => InlineAgentRef::Mock(b),
        }
    }

    pub fn model(&self) -> &str {
        self.as_ref().model()
    }

    pub fn upstream(&self) -> super::Upstream {
        self.as_ref().upstream()
    }

    pub fn output_mode(&self) -> super::OutputMode {
        self.as_ref().output_mode()
    }

    pub fn mcp_servers(&self) -> Option<&super::McpServers> {
        self.as_ref().mcp_servers()
    }

    pub fn client_objectiveai_mcp(
        &self,
    ) -> Option<&super::ClientObjectiveaiMcp> {
        self.as_ref().client_objectiveai_mcp()
    }

    pub fn prepare(&mut self) {
        match self {
            InlineAgentBase::Openrouter(b) => b.prepare(),
            InlineAgentBase::ClaudeAgentSdk(b) => b.prepare(),
            InlineAgentBase::CodexSdk(b) => b.prepare(),
            InlineAgentBase::Mock(b) => b.prepare(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match self {
            InlineAgentBase::Openrouter(b) => b.validate(),
            InlineAgentBase::ClaudeAgentSdk(b) => b.validate(),
            InlineAgentBase::CodexSdk(b) => b.validate(),
            InlineAgentBase::Mock(b) => b.validate(),
        }
    }

    pub fn id(&self) -> String {
        match self {
            InlineAgentBase::Openrouter(b) => b.id(),
            InlineAgentBase::ClaudeAgentSdk(b) => b.id(),
            InlineAgentBase::CodexSdk(b) => b.id(),
            InlineAgentBase::Mock(b) => b.id(),
        }
    }

    /// Validates and converts to an [`InlineAgent`] with computed ID.
    pub fn convert(self) -> Result<InlineAgent, String> {
        match self {
            InlineAgentBase::Openrouter(b) => {
                Ok(InlineAgent::Openrouter(b.try_into()?))
            }
            InlineAgentBase::ClaudeAgentSdk(b) => {
                Ok(InlineAgent::ClaudeAgentSdk(b.try_into()?))
            }
            InlineAgentBase::CodexSdk(b) => {
                Ok(InlineAgent::CodexSdk(b.try_into()?))
            }
            InlineAgentBase::Mock(b) => Ok(InlineAgent::Mock(b.try_into()?)),
        }
    }
}

/// A remote agent base definition with metadata.
///
/// Like [`InlineAgentBase`] but includes a description field for remote storage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.RemoteAgentBase")]
pub struct RemoteAgentBase {
    pub description: String,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<InlineAgentBase>")]
    pub inner: InlineAgentBase,
}

impl RemoteAgentBase {
    pub fn as_ref(&self) -> InlineAgentRef<'_> {
        self.inner.as_ref()
    }

    pub fn model(&self) -> &str {
        self.inner.model()
    }

    pub fn upstream(&self) -> super::Upstream {
        self.inner.upstream()
    }

    pub fn output_mode(&self) -> super::OutputMode {
        self.inner.output_mode()
    }

    pub fn mcp_servers(&self) -> Option<&super::McpServers> {
        self.inner.mcp_servers()
    }

    pub fn client_objectiveai_mcp(
        &self,
    ) -> Option<&super::ClientObjectiveaiMcp> {
        self.inner.client_objectiveai_mcp()
    }

    pub fn prepare(&mut self) {
        self.inner.prepare()
    }

    pub fn validate(&self) -> Result<(), String> {
        self.inner.validate()
    }

    pub fn id(&self) -> String {
        self.inner.id()
    }

    /// Validates and converts to a [`RemoteAgent`] with computed ID.
    pub fn convert(self) -> Result<RemoteAgent, String> {
        Ok(RemoteAgent {
            description: self.description,
            inner: self.inner.convert()?,
        })
    }
}

/// An Agent base definition, either remote (with metadata) or inline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.AgentBase")]
pub enum AgentBase {
    #[schemars(title = "Remote")]
    Remote(RemoteAgentBase),
    #[schemars(title = "Inline")]
    Inline(InlineAgentBase),
}

impl AgentBase {
    pub fn as_ref(&self) -> InlineAgentRef<'_> {
        match self {
            AgentBase::Remote(r) => r.as_ref(),
            AgentBase::Inline(i) => i.as_ref(),
        }
    }

    pub fn model(&self) -> &str {
        match self {
            AgentBase::Remote(r) => r.model(),
            AgentBase::Inline(i) => i.model(),
        }
    }

    pub fn upstream(&self) -> super::Upstream {
        match self {
            AgentBase::Remote(r) => r.upstream(),
            AgentBase::Inline(i) => i.upstream(),
        }
    }

    pub fn output_mode(&self) -> super::OutputMode {
        match self {
            AgentBase::Remote(r) => r.output_mode(),
            AgentBase::Inline(i) => i.output_mode(),
        }
    }

    pub fn mcp_servers(&self) -> Option<&super::McpServers> {
        match self {
            AgentBase::Remote(r) => r.mcp_servers(),
            AgentBase::Inline(i) => i.mcp_servers(),
        }
    }

    pub fn client_objectiveai_mcp(
        &self,
    ) -> Option<&super::ClientObjectiveaiMcp> {
        match self {
            AgentBase::Remote(r) => r.client_objectiveai_mcp(),
            AgentBase::Inline(i) => i.client_objectiveai_mcp(),
        }
    }

    pub fn prepare(&mut self) {
        match self {
            AgentBase::Remote(r) => r.prepare(),
            AgentBase::Inline(i) => i.prepare(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match self {
            AgentBase::Remote(r) => r.validate(),
            AgentBase::Inline(i) => i.validate(),
        }
    }

    pub fn id(&self) -> String {
        match self {
            AgentBase::Remote(r) => r.id(),
            AgentBase::Inline(i) => i.id(),
        }
    }

    /// Validates and converts to an [`Agent`] with computed ID.
    pub fn convert(self) -> Result<Agent, String> {
        match self {
            AgentBase::Remote(r) => Ok(Agent::Remote(r.convert()?)),
            AgentBase::Inline(i) => Ok(Agent::Inline(i.convert()?)),
        }
    }
}

// ── Ref types ──────────────────────────────────────────────────────

/// A borrowed reference into an [`InlineAgentBase`] variant.
#[derive(Clone, Copy, Debug)]
pub enum InlineAgentRef<'a> {
    Openrouter(&'a super::openrouter::AgentBase),
    ClaudeAgentSdk(&'a super::claude_agent_sdk::AgentBase),
    CodexSdk(&'a super::codex_sdk::AgentBase),
    Mock(&'a super::mock::AgentBase),
}

impl<'a> InlineAgentRef<'a> {
    pub fn to_owned(self) -> InlineAgentBase {
        match self {
            InlineAgentRef::Openrouter(b) => {
                InlineAgentBase::Openrouter(b.clone())
            }
            InlineAgentRef::ClaudeAgentSdk(b) => {
                InlineAgentBase::ClaudeAgentSdk(b.clone())
            }
            InlineAgentRef::CodexSdk(b) => InlineAgentBase::CodexSdk(b.clone()),
            InlineAgentRef::Mock(b) => InlineAgentBase::Mock(b.clone()),
        }
    }

    pub fn model(&self) -> &'a str {
        match self {
            InlineAgentRef::Openrouter(b) => &b.model,
            InlineAgentRef::ClaudeAgentSdk(b) => &b.model,
            InlineAgentRef::CodexSdk(b) => &b.model,
            InlineAgentRef::Mock(_) => super::mock::AgentBase::model(),
        }
    }

    pub fn upstream(&self) -> super::Upstream {
        match self {
            InlineAgentRef::Openrouter(_) => super::Upstream::Openrouter,
            InlineAgentRef::ClaudeAgentSdk(_) => {
                super::Upstream::ClaudeAgentSdk
            }
            InlineAgentRef::CodexSdk(_) => super::Upstream::CodexSdk,
            InlineAgentRef::Mock(_) => super::Upstream::Mock,
        }
    }

    pub fn output_mode(&self) -> super::OutputMode {
        match self {
            InlineAgentRef::Openrouter(b) => b.output_mode.into(),
            InlineAgentRef::ClaudeAgentSdk(b) => b.output_mode.into(),
            InlineAgentRef::CodexSdk(b) => b.output_mode.into(),
            InlineAgentRef::Mock(b) => b.output_mode.into(),
        }
    }

    pub fn mcp_servers(&self) -> Option<&'a super::McpServers> {
        match self {
            InlineAgentRef::Openrouter(b) => b.mcp_servers.as_ref(),
            InlineAgentRef::ClaudeAgentSdk(b) => b.mcp_servers.as_ref(),
            InlineAgentRef::CodexSdk(b) => b.mcp_servers.as_ref(),
            InlineAgentRef::Mock(b) => b.mcp_servers.as_ref(),
        }
    }

    pub fn client_objectiveai_mcp(
        &self,
    ) -> Option<&'a super::ClientObjectiveaiMcp> {
        match self {
            InlineAgentRef::Openrouter(b) => b.client_objectiveai_mcp.as_ref(),
            InlineAgentRef::ClaudeAgentSdk(b) => {
                b.client_objectiveai_mcp.as_ref()
            }
            InlineAgentRef::CodexSdk(b) => b.client_objectiveai_mcp.as_ref(),
            InlineAgentRef::Mock(b) => b.client_objectiveai_mcp.as_ref(),
        }
    }

    pub fn top_logprobs(&self) -> Option<u64> {
        match self {
            InlineAgentRef::Openrouter(b) => b.top_logprobs,
            InlineAgentRef::ClaudeAgentSdk(_) => None,
            InlineAgentRef::CodexSdk(_) => None,
            InlineAgentRef::Mock(b) => b.top_logprobs,
        }
    }

    pub fn merged_messages(
        &self,
        messages: Vec<super::completions::message::Message>,
    ) -> Vec<super::completions::message::Message> {
        match self {
            InlineAgentRef::Openrouter(b) => b.merged_messages(messages),
            InlineAgentRef::ClaudeAgentSdk(b) => b.merged_messages(messages),
            InlineAgentRef::CodexSdk(b) => b.merged_messages(messages),
            InlineAgentRef::Mock(b) => b.merged_messages(messages),
        }
    }
}

// ── Post-validation types (with computed ID) ───────────────────────

/// A validated inline Agent with its computed content-addressed ID.
///
/// This is an untagged enum that dispatches to the per-upstream Agent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.InlineAgent")]
pub enum InlineAgent {
    #[schemars(title = "Openrouter")]
    Openrouter(super::openrouter::Agent),
    #[schemars(title = "ClaudeAgentSdk")]
    ClaudeAgentSdk(super::claude_agent_sdk::Agent),
    #[schemars(title = "CodexSdk")]
    CodexSdk(super::codex_sdk::Agent),
    #[schemars(title = "Mock")]
    Mock(super::mock::Agent),
}

impl InlineAgent {
    pub fn id(&self) -> &str {
        match self {
            InlineAgent::Openrouter(a) => &a.id,
            InlineAgent::ClaudeAgentSdk(a) => &a.id,
            InlineAgent::CodexSdk(a) => &a.id,
            InlineAgent::Mock(a) => &a.id,
        }
    }

    pub fn base(&self) -> InlineAgentRef<'_> {
        match self {
            InlineAgent::Openrouter(a) => InlineAgentRef::Openrouter(&a.base),
            InlineAgent::ClaudeAgentSdk(a) => {
                InlineAgentRef::ClaudeAgentSdk(&a.base)
            }
            InlineAgent::CodexSdk(a) => InlineAgentRef::CodexSdk(&a.base),
            InlineAgent::Mock(a) => InlineAgentRef::Mock(&a.base),
        }
    }

    pub fn into_base(self) -> InlineAgentBase {
        match self {
            InlineAgent::Openrouter(a) => InlineAgentBase::Openrouter(a.base),
            InlineAgent::ClaudeAgentSdk(a) => {
                InlineAgentBase::ClaudeAgentSdk(a.base)
            }
            InlineAgent::CodexSdk(a) => InlineAgentBase::CodexSdk(a.base),
            InlineAgent::Mock(a) => InlineAgentBase::Mock(a.base),
        }
    }

    pub fn top_logprobs(&self) -> Option<u64> {
        self.base().top_logprobs()
    }
}

/// A validated remote Agent with metadata and computed content-addressed ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.RemoteAgent")]
pub struct RemoteAgent {
    pub description: String,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<InlineAgent>")]
    pub inner: InlineAgent,
}

impl RemoteAgent {
    pub fn id(&self) -> &str {
        self.inner.id()
    }

    pub fn base(&self) -> InlineAgentRef<'_> {
        self.inner.base()
    }

    pub fn into_base(self) -> RemoteAgentBase {
        RemoteAgentBase {
            description: self.description,
            inner: self.inner.into_base(),
        }
    }

    pub fn top_logprobs(&self) -> Option<u64> {
        self.inner.top_logprobs()
    }
}

/// A validated Agent, either remote (with metadata) or inline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.Agent")]
pub enum Agent {
    #[schemars(title = "Remote")]
    Remote(RemoteAgent),
    #[schemars(title = "Inline")]
    Inline(InlineAgent),
}

impl Agent {
    pub fn id(&self) -> &str {
        match self {
            Agent::Remote(a) => a.inner.id(),
            Agent::Inline(a) => a.id(),
        }
    }

    pub fn base(&self) -> InlineAgentRef<'_> {
        match self {
            Agent::Remote(a) => a.inner.base(),
            Agent::Inline(a) => a.base(),
        }
    }

    pub fn into_base(self) -> AgentBase {
        match self {
            Agent::Remote(a) => AgentBase::Remote(RemoteAgentBase {
                description: a.description,
                inner: a.inner.into_base(),
            }),
            Agent::Inline(a) => AgentBase::Inline(a.into_base()),
        }
    }

    pub fn top_logprobs(&self) -> Option<u64> {
        self.base().top_logprobs()
    }
}

// ── WithFallbacks types ────────────────────────────────────────────

/// An [`InlineAgentBase`] with optional fallbacks (no description).
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.InlineAgentBaseWithFallbacks")]
pub struct InlineAgentBaseWithFallbacks {
    /// The primary agent configuration.
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<InlineAgentBase>")]
    pub inner: InlineAgentBase,
    /// Fallback agents to try if the primary fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub fallbacks: Option<Vec<InlineAgentBase>>,
}

impl InlineAgentBaseWithFallbacks {
    /// Validates and converts to an [`InlineAgentWithFallbacks`] with computed IDs.
    pub fn convert(self) -> Result<InlineAgentWithFallbacks, String> {
        let inner = self.inner.convert()?;
        let fallbacks = match self.fallbacks {
            Some(fallbacks) if !fallbacks.is_empty() => {
                let mut converted = Vec::with_capacity(fallbacks.len());
                for fallback in fallbacks {
                    converted.push(fallback.convert()?);
                }
                Some(converted)
            }
            _ => None,
        };
        Ok(InlineAgentWithFallbacks { inner, fallbacks })
    }
}

/// A validated [`InlineAgent`] with optional fallbacks (no description).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.InlineAgentWithFallbacks")]
pub struct InlineAgentWithFallbacks {
    /// The primary agent configuration.
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<InlineAgent>")]
    pub inner: InlineAgent,
    /// Fallback agents to try if the primary fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub fallbacks: Option<Vec<InlineAgent>>,
}

impl InlineAgentWithFallbacks {
    /// Returns the concatenated IDs of the primary agent and all fallbacks.
    ///
    /// Used by swarms to compute their content-addressed ID.
    pub fn full_id(&self) -> String {
        match &self.fallbacks {
            Some(fallbacks) => {
                let id = self.inner.id();
                let mut full_id =
                    String::with_capacity(id.len() + fallbacks.len() * 22);
                full_id.push_str(id);
                for fallback in fallbacks {
                    full_id.push_str(fallback.id());
                }
                full_id
            }
            None => self.inner.id().to_owned(),
        }
    }

    /// Returns an iterator over the IDs of the primary agent and all fallbacks.
    pub fn ids(&self) -> impl Iterator<Item = &str> + Send {
        std::iter::once(self.inner.id()).chain(
            self.fallbacks.as_ref().into_iter().flat_map(|fallbacks| {
                fallbacks.iter().map(|fallback| fallback.id())
            }),
        )
    }
}

/// A remote agent base definition with description and optional fallbacks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.RemoteAgentBaseWithFallbacks")]
pub struct RemoteAgentBaseWithFallbacks {
    pub description: String,
    #[serde(flatten)]
    #[schemars(
        schema_with = "crate::flatten_schema::<InlineAgentBaseWithFallbacks>"
    )]
    pub inner: InlineAgentBaseWithFallbacks,
}

impl RemoteAgentBaseWithFallbacks {
    /// Validates and converts to a [`RemoteAgentWithFallbacks`].
    pub fn convert(self) -> Result<RemoteAgentWithFallbacks, String> {
        Ok(RemoteAgentWithFallbacks {
            description: self.description,
            inner: self.inner.convert()?,
        })
    }
}

/// A validated remote agent with description and optional fallbacks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.RemoteAgentWithFallbacks")]
pub struct RemoteAgentWithFallbacks {
    pub description: String,
    #[serde(flatten)]
    #[schemars(
        schema_with = "crate::flatten_schema::<InlineAgentWithFallbacks>"
    )]
    pub inner: InlineAgentWithFallbacks,
}

impl RemoteAgentWithFallbacks {
    /// Returns the concatenated IDs of the primary agent and all fallbacks.
    pub fn full_id(&self) -> String {
        self.inner.full_id()
    }

    /// Returns an iterator over the IDs of the primary agent and all fallbacks.
    pub fn ids(&self) -> impl Iterator<Item = &str> + Send {
        self.inner.ids()
    }
}

/// A validated agent with optional fallbacks, either remote (with description) or inline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.AgentWithFallbacks")]
pub enum AgentWithFallbacks {
    #[schemars(title = "Remote")]
    Remote(RemoteAgentWithFallbacks),
    #[schemars(title = "Inline")]
    Inline(InlineAgentWithFallbacks),
}

impl AgentWithFallbacks {
    /// Returns the inner `InlineAgentWithFallbacks` regardless of variant.
    pub fn inline(&self) -> &InlineAgentWithFallbacks {
        match self {
            AgentWithFallbacks::Remote(a) => &a.inner,
            AgentWithFallbacks::Inline(a) => a,
        }
    }

    /// Returns the primary agent.
    pub fn agent(&self) -> &InlineAgent {
        &self.inline().inner
    }

    /// Returns the fallback agents.
    pub fn fallbacks(&self) -> Option<&Vec<InlineAgent>> {
        self.inline().fallbacks.as_ref()
    }

    /// Returns the concatenated IDs of the primary agent and all fallbacks.
    pub fn full_id(&self) -> String {
        self.inline().full_id()
    }

    /// Returns an iterator over the IDs of the primary agent and all fallbacks.
    pub fn ids(&self) -> impl Iterator<Item = &str> + Send {
        self.inline().ids()
    }

    pub fn id(&self) -> &str {
        self.agent().id()
    }

    pub fn base(&self) -> InlineAgentRef<'_> {
        self.agent().base()
    }

    pub fn top_logprobs(&self) -> Option<u64> {
        self.agent().top_logprobs()
    }
}

// ── InlineAgentBaseWithFallbacksOrRemote ─────────────────────────────────

/// An agent specification that is either an inline agent base with fallbacks
/// or a remote path reference.
///
/// Used in swarm definitions to allow agents to be specified inline
/// (with optional fallbacks) or resolved from a remote source via a
/// hashmap during conversion.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(untagged)]
#[schemars(rename = "agent.InlineAgentBaseWithFallbacksOrRemote")]
pub enum InlineAgentBaseWithFallbacksOrRemote {
    #[schemars(title = "AgentBase")]
    AgentBase(InlineAgentBaseWithFallbacks),
    #[schemars(title = "Remote")]
    Remote(crate::RemotePath),
}

/// Like [`InlineAgentBaseWithFallbacksOrRemote`] but with optional commit.
/// Used in request types where commit resolution happens server-side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.InlineAgentBaseWithFallbacksOrRemoteCommitOptional")]
pub enum InlineAgentBaseWithFallbacksOrRemoteCommitOptional {
    #[schemars(title = "AgentBase")]
    AgentBase(InlineAgentBaseWithFallbacks),
    #[schemars(title = "Remote")]
    Remote(crate::RemotePathCommitOptional),
}

// ── WithCount types (for swarm agent slots) ────────────────────────

fn default_count() -> u64 {
    1
}

/// An [`InlineAgentBaseWithFallbacksOrRemote`] with a count
/// (pre-validation swarm agent slot).
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.InlineAgentBaseWithFallbacksOrRemoteWithCount")]
pub struct InlineAgentBaseWithFallbacksOrRemoteWithCount {
    /// Number of instances of this agent in the swarm. Defaults to 1.
    #[serde(default = "default_count")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub count: u64,
    /// The agent configuration.
    #[serde(flatten)]
    #[schemars(
        schema_with = "crate::flatten_schema::<InlineAgentBaseWithFallbacksOrRemote>"
    )]
    pub inner: InlineAgentBaseWithFallbacksOrRemote,
}

/// An [`AgentWithFallbacks`] with a count (post-validation swarm agent slot).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.AgentWithFallbacksWithCount")]
pub struct AgentWithFallbacksWithCount {
    /// Number of instances of this agent in the swarm. Defaults to 1.
    #[serde(default = "default_count")]
    pub count: u64,
    /// The validated agent configuration.
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<AgentWithFallbacks>")]
    pub inner: AgentWithFallbacks,
}
