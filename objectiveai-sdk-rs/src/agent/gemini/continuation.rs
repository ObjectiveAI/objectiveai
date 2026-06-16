use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.gemini.Continuation")]
pub struct Continuation {
    pub upstream: super::Upstream,
    /// Full slash-separated lineage of the agent this continuation
    /// belongs to (e.g. `A/B/agtcpl-<uuid>-<created>`). Minted on the
    /// agent's first spawn and preserved verbatim across every
    /// continuation round so the agent's identity stays stable
    /// regardless of who resumes the conversation.
    pub agent_instance_hierarchy: String,
    pub session_id: String,
    pub mcp_sessions: indexmap::IndexMap<String, String>,
}
