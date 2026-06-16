use indexmap::IndexMap;
use serde::Serialize;

use super::super::McpServerConfig;

/// Wire shape of the `params` object on a `run` request. Mirrors the
/// gemini runner's expected schema 1:1; the field names here must match
/// what `handle_run` reads.
///
/// The runner is STATELESS — there is no `resume`/`thread_id`. `messages`
/// carries the FULL conversation (prior continuation history + this
/// turn's messages) on every call.
#[derive(Debug, Serialize)]
pub struct RunParams<'a> {
    pub model: &'a str,

    /// The full conversation in the runner's message shape.
    pub messages: &'a [objectiveai_sdk::agent::gemini::Message],

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<&'a str>,

    /// `"low" | "medium" | "high"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<&'a str>,

    /// Whether extended thinking is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_enabled: Option<bool>,

    /// HTTP MCP servers — name → `{url, headers}`. Empty map = no MCP.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub mcp_servers: &'a IndexMap<String, McpServerConfig>,

    /// Composite agent id forwarded by the api at MCP-connect time
    /// (`X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`). The Python runner
    /// forwards this to the MCP proxy / SDK as
    /// `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_instance_hierarchy: Option<&'a str>,
}
