/// Agent identity + response routing args carried per call.
///
/// When passed as `Some(&AgentArguments)` to a [`CommandExecutor`],
/// subprocess-spawning executors (e.g. [`binary::BinaryExecutor`])
/// apply ALL nine fields to the spawned child's env atomically —
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
/// - `plugin_owner` ↔ `OBJECTIVEAI_PLUGIN_OWNER`
/// - `plugin_repository` ↔ `OBJECTIVEAI_PLUGIN_REPOSITORY`
/// - `plugin_version` ↔ `OBJECTIVEAI_PLUGIN_VERSION`
///
/// The three `plugin_*` fields are the PLUGIN CALLER identity —
/// which installed plugin originated this request. UNSPOOFABLE by
/// design: the daemon's own `plugins run` is the only writer (it
/// stamps the nested command scope in-process and the plugin child's
/// env informationally); wire requests and the CLI environment can
/// NEVER assert them — the daemon ignores any inbound claim. They
/// appear only in daemon-AUTHORED payloads (e.g. user requests).
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
    pub plugin_owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_version: Option<String>,
}

impl AgentArguments {
    /// Build the bag from a proxy-session transient-header map — the
    /// six `X-OBJECTIVEAI-*` keys the API stamps on every proxy
    /// connect. Key lookup is case-insensitive. The `plugin_*` trio
    /// is NEVER read from headers: plugin identity rides its own
    /// typed wire field (the proxy→daemon `Command` payload) or is
    /// stamped in-process by `plugins run` — a header claim is
    /// ignored.
    pub fn from_transient_headers(
        headers: &indexmap::IndexMap<String, String>,
    ) -> Self {
        let get = |name: &str| {
            headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone())
        };
        Self {
            agent_instance_hierarchy: get(
                "X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY",
            ),
            agent_id: get("X-OBJECTIVEAI-AGENT-ID"),
            agent_full_id: get("X-OBJECTIVEAI-AGENT-FULL-ID"),
            agent_remote: get("X-OBJECTIVEAI-AGENT-REMOTE"),
            response_id: get("X-OBJECTIVEAI-RESPONSE-ID"),
            response_ids: get("X-OBJECTIVEAI-RESPONSE-IDS"),
            plugin_owner: None,
            plugin_repository: None,
            plugin_version: None,
        }
    }

    /// The IDENTITY environment pairs — every `Some` field's
    /// `(env name, value)`, agent identity AND the plugin trio. The
    /// names are exactly [`crate::agent::RESERVED_LABORATORY_ENV`]:
    /// the laboratory host stamps these onto every ephemeral
    /// (agent/plugin) container at create, which is why
    /// agent-laboratory `env` declarations may not use them. The
    /// plugin trio is `Some` only when an AUTHORITY set it in-process
    /// (wire-parsed bags always carry `None` there — see
    /// [`Self::from_transient_headers`]).
    pub fn identity_env(&self) -> Vec<(String, String)> {
        [
            (
                "OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY",
                &self.agent_instance_hierarchy,
            ),
            ("OBJECTIVEAI_AGENT_ID", &self.agent_id),
            ("OBJECTIVEAI_AGENT_FULL_ID", &self.agent_full_id),
            ("OBJECTIVEAI_AGENT_REMOTE", &self.agent_remote),
            ("OBJECTIVEAI_RESPONSE_ID", &self.response_id),
            ("OBJECTIVEAI_RESPONSE_IDS", &self.response_ids),
            ("OBJECTIVEAI_PLUGIN_OWNER", &self.plugin_owner),
            ("OBJECTIVEAI_PLUGIN_REPOSITORY", &self.plugin_repository),
            ("OBJECTIVEAI_PLUGIN_VERSION", &self.plugin_version),
        ]
        .into_iter()
        .filter_map(|(name, value)| {
            value.as_ref().map(|v| (name.to_string(), v.clone()))
        })
        .collect()
    }

    /// Apply this bag to a child-process command: every `Some(v)`
    /// stamps the matching env var, every `None` env-removes it so
    /// the parent's value can't leak through. Called by
    /// [`binary::BinaryExecutor`]; available for any executor that
    /// spawns a subprocess.
    #[cfg(feature = "cli-executor")]
    pub fn apply_to_command(&self, command: &mut tokio::process::Command) {
        let pairs: [(&str, &Option<String>); 9] = [
            (
                "OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY",
                &self.agent_instance_hierarchy,
            ),
            ("OBJECTIVEAI_AGENT_ID", &self.agent_id),
            ("OBJECTIVEAI_AGENT_FULL_ID", &self.agent_full_id),
            ("OBJECTIVEAI_AGENT_REMOTE", &self.agent_remote),
            ("OBJECTIVEAI_RESPONSE_ID", &self.response_id),
            ("OBJECTIVEAI_RESPONSE_IDS", &self.response_ids),
            ("OBJECTIVEAI_PLUGIN_OWNER", &self.plugin_owner),
            ("OBJECTIVEAI_PLUGIN_REPOSITORY", &self.plugin_repository),
            ("OBJECTIVEAI_PLUGIN_VERSION", &self.plugin_version),
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
