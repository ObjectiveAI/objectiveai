use indexmap::IndexMap;
use serde::Serialize;

use super::super::mcp_server_config::McpServerConfig;
use super::super::sdk_message::SDKUserMessage;

/// Wire shape of the `params` object on a `run` request. Mirrors the
/// Python runner's expected schema 1:1; the field names here must
/// match what `handle_run` reads.
#[derive(Debug, Serialize)]
pub struct RunParams<'a> {
    pub model: &'a str,
    pub message: &'a SDKUserMessage,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<objectiveai_sdk::agent::claude_agent_sdk::Effort>,

    #[serde(skip_serializing_if = "is_false")]
    pub thinking_disabled: bool,

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub mcp_servers: &'a IndexMap<String, McpServerConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<&'a str>,

    pub rate_limit_max_retries: u64,
    pub rate_limit_max_wait_secs: u64,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}
