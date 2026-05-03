use indexmap::IndexMap;
use serde::Serialize;

use super::super::McpServerConfig;
use super::super::ModelReasoningEffort;
use super::super::RunnerUserMessage;

/// Wire shape of the `params` object on a `run` request. Mirrors the
/// Python runner's expected schema 1:1; the field names here must
/// match what `_run_one` reads.
#[derive(Debug, Serialize)]
pub struct RunParams<'a> {
    pub model: &'a str,
    pub input: &'a RunnerUserMessage,
    pub cwd: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<ModelReasoningEffort>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<&'a str>,

    /// HTTP MCP servers — name → config. Empty map = no MCP. The
    /// runner currently ignores this field; wiring it into
    /// `Codex.Thread` is a follow-up.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub mcp_servers: &'a IndexMap<String, McpServerConfig>,
}
