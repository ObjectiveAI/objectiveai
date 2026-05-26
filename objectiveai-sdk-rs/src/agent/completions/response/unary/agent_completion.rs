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
    /// `true` when the MCP proxy holds queued messages that were not
    /// delivered to the agent via a tool response on this turn. See
    /// [`super::streaming::AgentCompletionChunk::messages_queued`].
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub messages_queued: Option<bool>,
}

impl AgentCompletion {
    /// Normalize non-deterministic fields for test snapshot comparison.
    pub fn normalize_for_tests(&mut self) {
        use crate::agent::completions::message::{RichContent, RichContentPart};

        self.id = String::new();
        self.created = 0;
        for msg in &mut self.messages {
            match msg {
                super::Message::Assistant(asst) => {
                    asst.upstream_id = String::new();
                    asst.created = 0;
                }
                super::Message::Tool(tool) => {
                    // Strip the `agent_id` key the CLI's
                    // `cli::output::Handle::emit` stamps on every
                    // emitted JSON line. The value is the racy
                    // `next_agent_index` counter — the order-of-task
                    // assignment varies run-to-run, so leaving it in
                    // would break snapshot determinism. Walk text
                    // payloads, parse each line, drop `agent_id`,
                    // re-serialize.
                    match &mut tool.inner.content {
                        RichContent::Text(s) => {
                            *s = strip_agent_id_lines(s);
                        }
                        RichContent::Parts(parts) => {
                            for p in parts {
                                if let RichContentPart::Text { text } = p {
                                    *text = strip_agent_id_lines(text);
                                }
                            }
                        }
                    }
                }
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

/// Private mirror of `cli::output::strip_agent_id_lines` for use inside
/// `normalize_for_tests`. The shared public copy lives behind the
/// `cli` feature flag and the SDK core builds without that feature,
/// so we keep a small local duplicate here. ~15 lines, no shared
/// state, no drift risk.
fn strip_agent_id_lines(text: &str) -> String {
    let mut out: String = text
        .lines()
        .map(|line| {
            // Only rewrite lines that parse as a JSON *object* AND
            // actually contain an `agent_id` top-level key. See the
            // sibling helper in `cli::output::strip_agent_id_lines`
            // for the indentation-preservation rationale.
            let Ok(serde_json::Value::Object(mut obj)) =
                serde_json::from_str::<serde_json::Value>(line)
            else {
                return line.to_string();
            };
            if obj.remove("agent_id").is_none() {
                return line.to_string();
            }
            serde_json::to_string(&serde_json::Value::Object(obj))
                .unwrap_or_else(|_| line.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
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
            messages_queued,
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
            messages_queued,
        }
    }
}
