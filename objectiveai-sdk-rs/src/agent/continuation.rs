use base64::Engine;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Continuation state for multi-turn agent completions.
///
/// Returned in the final streaming chunk and in unary responses.
/// Pass it back in the next request to continue the conversation.
/// Serialized as base64-encoded JSON.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.Continuation")]
pub enum Continuation {
    #[schemars(title = "Openrouter")]
    Openrouter(super::openrouter::Continuation),
    #[schemars(title = "ClaudeAgentSdk")]
    ClaudeAgentSdk(super::claude_agent_sdk::Continuation),
    #[schemars(title = "CodexSdk")]
    CodexSdk(super::codex_sdk::Continuation),
    #[schemars(title = "Mock")]
    Mock(super::mock::Continuation),
    #[schemars(title = "Script")]
    Script(super::script::Continuation),
}

impl From<super::openrouter::Continuation> for Continuation {
    fn from(inner: super::openrouter::Continuation) -> Self {
        Self::Openrouter(inner)
    }
}

impl From<super::claude_agent_sdk::Continuation> for Continuation {
    fn from(inner: super::claude_agent_sdk::Continuation) -> Self {
        Self::ClaudeAgentSdk(inner)
    }
}

impl From<super::codex_sdk::Continuation> for Continuation {
    fn from(inner: super::codex_sdk::Continuation) -> Self {
        Self::CodexSdk(inner)
    }
}

impl From<super::mock::Continuation> for Continuation {
    fn from(inner: super::mock::Continuation) -> Self {
        Self::Mock(inner)
    }
}

impl From<super::script::Continuation> for Continuation {
    fn from(inner: super::script::Continuation) -> Self {
        Self::Script(inner)
    }
}

impl Continuation {
    /// Full slash-separated lineage of the agent this continuation
    /// belongs to. See per-upstream struct docs for the semantic.
    /// Empty string if the continuation was minted before this field
    /// existed (pre-field tokens deserialize the default).
    pub fn agent_instance_hierarchy(&self) -> &str {
        match self {
            Self::Openrouter(c) => c.agent_instance_hierarchy.as_str(),
            Self::ClaudeAgentSdk(c) => c.agent_instance_hierarchy.as_str(),
            Self::CodexSdk(c) => c.agent_instance_hierarchy.as_str(),
            Self::Mock(c) => c.agent_instance_hierarchy.as_str(),
            Self::Script(c) => c.agent_instance_hierarchy.as_str(),
        }
    }

    /// Stamps `agent_instance_hierarchy` on the outgoing continuation.
    /// The api server calls this just before returning the wire
    /// continuation so the next round inherits the same lineage.
    pub fn set_agent_instance_hierarchy(&mut self, id: String) {
        match self {
            Self::Openrouter(c) => c.agent_instance_hierarchy = id,
            Self::ClaudeAgentSdk(c) => c.agent_instance_hierarchy = id,
            Self::CodexSdk(c) => c.agent_instance_hierarchy = id,
            Self::Mock(c) => c.agent_instance_hierarchy = id,
            Self::Script(c) => c.agent_instance_hierarchy = id,
        }
    }

    /// Returns the upstream type for this continuation.
    pub fn upstream(&self) -> super::Upstream {
        match self {
            Self::Openrouter(_) => super::Upstream::Openrouter,
            Self::ClaudeAgentSdk(_) => super::Upstream::ClaudeAgentSdk,
            Self::CodexSdk(_) => super::Upstream::CodexSdk,
            Self::Mock(_) => super::Upstream::Mock,
            Self::Script(_) => super::Upstream::Script,
        }
    }

    /// Serializes the continuation to a base64-encoded string.
    pub fn to_string(&self) -> String {
        let json = serde_json::to_string(self).unwrap();
        base64::engine::general_purpose::STANDARD.encode(json)
    }

    /// Attempts to deserialize a continuation from a base64-encoded string.
    pub fn try_from_string(s: &str) -> Option<Self> {
        let json = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
        let continuation = serde_json::from_slice(&json).ok()?;
        Some(continuation)
    }
}
