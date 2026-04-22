use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.claude_code.Continuation")]
pub struct Continuation {
    pub upstream: super::Upstream,
    pub session_id: String,
    pub mcp_sessions: indexmap::IndexMap<String, String>,
}
