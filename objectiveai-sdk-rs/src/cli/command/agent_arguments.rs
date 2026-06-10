/// Agent identity + response routing args carried per call.
///
/// When passed as `Some(&AgentArguments)` to a [`CommandExecutor`],
/// subprocess-spawning executors (e.g. [`binary::BinaryExecutor`])
/// apply ALL six fields to the spawned child's env atomically —
/// `Some(v)` → set, `None` → `env_remove` so the parent's value for
/// that var can't leak through. `None` for the whole bag means
/// "inherit parent env unmodified". In-process executors (e.g.
/// [`plugin::PluginExecutor`]) ignore the bag.
///
/// Field ↔ env-var mapping (same as `EnvConfigBuilder` in
/// `objectiveai-cli/src/run.rs`):
///
/// - `agent_instance_hierarchy` ↔ `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY`
/// - `agent_id` ↔ `OBJECTIVEAI_AGENT_ID`
/// - `agent_full_id` ↔ `OBJECTIVEAI_AGENT_FULL_ID`
/// - `agent_remote` ↔ `OBJECTIVEAI_AGENT_REMOTE`
/// - `response_id` ↔ `OBJECTIVEAI_RESPONSE_ID`
/// - `response_ids` ↔ `OBJECTIVEAI_RESPONSE_IDS`
/// - `mcp_session_id` ↔ `MCP_SESSION_ID` (the MCP transport
///   session id minted by the MCP server, NOT an objectiveai-scoped
///   identifier — same env-var convention as
///   [`crate::mcp::MCP_SESSION_ID_ENV`])
#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[schemars(rename = "cli.command.AgentArguments")]
pub struct AgentArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_instance_hierarchy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_full_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_remote: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_ids: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_session_id: Option<String>,
}

impl AgentArguments {
    /// Apply this bag to a child-process command: every `Some(v)`
    /// stamps the matching env var, every `None` env-removes it so
    /// the parent's value can't leak through. Called by
    /// [`binary::BinaryExecutor`]; available for any executor that
    /// spawns a subprocess.
    #[cfg(feature = "cli-executor")]
    pub fn apply_to_command(&self, command: &mut tokio::process::Command) {
        let pairs: [(&str, &Option<String>); 7] = [
            (
                "OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY",
                &self.agent_instance_hierarchy,
            ),
            ("OBJECTIVEAI_AGENT_ID", &self.agent_id),
            ("OBJECTIVEAI_AGENT_FULL_ID", &self.agent_full_id),
            ("OBJECTIVEAI_AGENT_REMOTE", &self.agent_remote),
            ("OBJECTIVEAI_RESPONSE_ID", &self.response_id),
            ("OBJECTIVEAI_RESPONSE_IDS", &self.response_ids),
            (crate::mcp::MCP_SESSION_ID_ENV, &self.mcp_session_id),
        ];
        for (name, value) in pairs {
            match value {
                Some(v) => {
                    command.env(name, v);
                }
                None => {
                    command.env_remove(name);
                }
            }
        }
    }
}
