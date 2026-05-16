use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.openrouter.Continuation")]
pub struct Continuation {
    pub upstream: super::Upstream,
    pub messages: Vec<super::super::completions::message::Message>,
    pub mcp_sessions: indexmap::IndexMap<String, String>,
}
