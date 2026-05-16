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

impl Continuation {
    /// Returns the MCP sessions map for this continuation.
    pub fn mcp_sessions(&self) -> &indexmap::IndexMap<String, String> {
        match self {
            Self::Openrouter(c) => &c.mcp_sessions,
            Self::ClaudeAgentSdk(c) => &c.mcp_sessions,
            Self::CodexSdk(c) => &c.mcp_sessions,
            Self::Mock(c) => &c.mcp_sessions,
        }
    }

    /// Returns the upstream type for this continuation.
    pub fn upstream(&self) -> super::Upstream {
        match self {
            Self::Openrouter(_) => super::Upstream::Openrouter,
            Self::ClaudeAgentSdk(_) => super::Upstream::ClaudeAgentSdk,
            Self::CodexSdk(_) => super::Upstream::CodexSdk,
            Self::Mock(_) => super::Upstream::Mock,
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
