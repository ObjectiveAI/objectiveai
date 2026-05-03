//! Agent completion response type.

use crate::agent::completions::response;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A complete agent completion response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(rename = "agent.completions.response.unary.AgentCompletion")]
pub struct AgentCompletion {
    pub id: String,
    pub created: u64,
    pub messages: Vec<super::Message>,
    /// The object type (always "agent.completion").
    pub object: super::Object,
    pub usage: response::Usage,
    /// Upstream provider
    pub upstream: crate::agent::Upstream,
    /// Error details if this completion failed.
    pub error: Option<crate::error::ResponseError>,
    /// Continuation state for multi-turn conversations.
    pub continuation: Option<String>,
}

impl AgentCompletion {
    /// Normalize non-deterministic fields for test snapshot comparison.
    pub fn normalize_for_tests(&mut self) {
        self.id = String::new();
        self.created = 0;
        for msg in &mut self.messages {
            if let super::Message::Assistant(asst) = msg {
                asst.upstream_id = String::new();
                asst.created = 0;
            }
        }

        // The continuation is base64-encoded JSON whose payload includes
        // an `mcp_sessions` map keyed by proxy URL with freshly-minted
        // session UUIDs as values. Both the URL's port and the UUIDs are
        // random per run — they'd break every snapshot otherwise.
        // Decode, clear the map, re-encode.
        if let Some(s) = &mut self.continuation {
            if let Some(mut c) = crate::agent::Continuation::try_from_string(s) {
                match &mut c {
                    crate::agent::Continuation::Openrouter(x) => {
                        x.mcp_sessions.clear()
                    }
                    crate::agent::Continuation::ClaudeAgentSdk(x) => {
                        x.mcp_sessions.clear()
                    }
                    crate::agent::Continuation::CodexSdk(x) => {
                        x.mcp_sessions.clear()
                    }
                    crate::agent::Continuation::Mock(x) => {
                        x.mcp_sessions.clear()
                    }
                }
                *s = c.to_string();
            }
        }
    }
}

impl From<response::streaming::AgentCompletionChunk> for AgentCompletion {
    fn from(
        response::streaming::AgentCompletionChunk {
            id,
            created,
            messages,
            object,
            usage,
            upstream,
            error,
            continuation,
        }: response::streaming::AgentCompletionChunk,
    ) -> Self {
        Self {
            id,
            created,
            messages: messages.into_iter().map(Into::into).collect(),
            object: object.into(),
            usage: usage.unwrap_or_default(),
            upstream,
            error,
            continuation,
        }
    }
}
