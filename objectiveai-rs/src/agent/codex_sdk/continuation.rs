use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.codex_sdk.Continuation")]
pub struct Continuation {
    pub upstream: super::Upstream,
    pub thread_id: String,
    pub mcp_sessions: indexmap::IndexMap<String, String>,
}
