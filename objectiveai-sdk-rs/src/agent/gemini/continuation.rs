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
    /// Full prior conversation in the canonical agent-completions
    /// message shape. The Gemini runner is stateless (no server-side
    /// session resume), so the API persists the entire history here and
    /// replays it — prior history followed by the next turn's messages —
    /// on every continuation round. The API translates these canonical
    /// messages into the runner's own wire shape at request time. Empty
    /// on a fresh conversation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub messages: Vec<super::super::completions::message::Message>,
}
