use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.claude_agent_sdk.Continuation")]
pub struct Continuation {
    pub upstream: super::Upstream,
    /// Full slash-separated lineage of the agent this continuation
    /// belongs to (e.g. `A/B/agtcpl-<uuid>-<created>`). Minted on the
    /// agent's first spawn and preserved verbatim across every
    /// continuation round so the agent's identity stays stable
    /// regardless of who resumes the conversation. `#[serde(default)]`
    /// so pre-field tokens deserialize as empty and get re-minted on
    /// resume.
    #[serde(default)]
    pub agent_id: String,
    pub session_id: String,
    pub mcp_sessions: indexmap::IndexMap<String, String>,
    /// Per-agent reverse-attach session id baked into this agent's
    /// `client_objectiveai_mcp` proxy URL path segment. Persisted
    /// across continuation resumes so the proxy URLs stored in
    /// `mcp_sessions` keep matching the registered WS reverse-attach
    /// route. `None` when this agent never used `client_objectiveai_mcp`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub ws_session_id: Option<String>,
}
