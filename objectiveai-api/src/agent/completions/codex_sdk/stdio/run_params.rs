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
    /// runner materializes a per-request `CODEX_HOME/config.toml`
    /// from this map and points the codex subprocess at it via the
    /// `CODEX_HOME` env var.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub mcp_servers: &'a IndexMap<String, McpServerConfig>,

    /// Composite agent id forwarded by the api at MCP-connect time
    /// (`X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`). The Python runner sets this as
    /// `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY` in the per-Run env dict it hands to the
    /// codex subprocess via `CodexOptions(env=...)`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_instance_hierarchy: Option<&'a str>,
}
