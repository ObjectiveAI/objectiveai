//! The configuration vocabularies control requests carry.

use serde::Deserialize;

use super::super::system::PermissionMode;

/// A permission-rule change — the source's six kinds, camel-cased on
/// the wire. Untagged, each variant carrying its `type` literal as a
/// marker field.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum PermissionUpdate {
    /// Add rules with a behavior, somewhere.
    AddRules {
        /// Always `addRules`.
        r#type: AddRulesType,
        /// The rules.
        rules: Vec<PermissionRuleValue>,
        /// What they decide.
        behavior: PermissionBehavior,
        /// Which settings layer they land in.
        destination: PermissionUpdateDestination,
    },
    /// Replace the rules with these.
    ReplaceRules {
        /// Always `replaceRules`.
        r#type: ReplaceRulesType,
        /// The rules.
        rules: Vec<PermissionRuleValue>,
        /// What they decide.
        behavior: PermissionBehavior,
        /// Which settings layer they land in.
        destination: PermissionUpdateDestination,
    },
    /// Remove these rules.
    RemoveRules {
        /// Always `removeRules`.
        r#type: RemoveRulesType,
        /// The rules.
        rules: Vec<PermissionRuleValue>,
        /// What they decided.
        behavior: PermissionBehavior,
        /// Which settings layer they leave.
        destination: PermissionUpdateDestination,
    },
    /// Change the permission mode.
    SetMode {
        /// Always `setMode`.
        r#type: SetModeType,
        /// The new mode.
        mode: PermissionMode,
        /// Which settings layer it lands in.
        destination: PermissionUpdateDestination,
    },
    /// Grant access to directories.
    AddDirectories {
        /// Always `addDirectories`.
        r#type: AddDirectoriesType,
        /// The directories.
        directories: Vec<String>,
        /// Which settings layer they land in.
        destination: PermissionUpdateDestination,
    },
    /// Revoke access to directories.
    RemoveDirectories {
        /// Always `removeDirectories`.
        r#type: RemoveDirectoriesType,
        /// The directories.
        directories: Vec<String>,
        /// Which settings layer they leave.
        destination: PermissionUpdateDestination,
    },
}

/// One permission rule: a tool, and optionally a narrowing pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct PermissionRuleValue {
    /// The tool the rule is about.
    #[serde(rename = "toolName")]
    pub tool_name: String,
    /// The rule's content — the pattern after the tool's name.
    #[serde(
        rename = "ruleContent"
    )]
    pub rule_content: Option<String>,
}

/// What a permission rule decides.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PermissionBehavior {
    /// Allowed.
    Allow,
    /// Denied.
    Deny,
    /// Ask the user.
    Ask,
}

/// Which settings layer a permission update lands in.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PermissionUpdateDestination {
    /// The user's settings.
    #[serde(rename = "userSettings")]
    UserSettings,
    /// The project's shared settings.
    #[serde(rename = "projectSettings")]
    ProjectSettings,
    /// The project's gitignored settings.
    #[serde(rename = "localSettings")]
    LocalSettings,
    /// This session only.
    #[serde(rename = "session")]
    Session,
    /// The command line.
    #[serde(rename = "cliArg")]
    CliArg,
}

/// The lifecycle events a hook can register for — the source's
/// `HOOK_EVENTS`, whole.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
pub enum HookEvent {
    /// Before a tool runs.
    PreToolUse,
    /// After a tool runs.
    PostToolUse,
    /// After a tool fails.
    PostToolUseFailure,
    /// A notification fired.
    Notification,
    /// A prompt was submitted.
    UserPromptSubmit,
    /// A session started.
    SessionStart,
    /// A session ended.
    SessionEnd,
    /// The turn stopped.
    Stop,
    /// The turn stopped on a failure.
    StopFailure,
    /// A subagent started.
    SubagentStart,
    /// A subagent stopped.
    SubagentStop,
    /// Before compaction.
    PreCompact,
    /// After compaction.
    PostCompact,
    /// A permission is being asked.
    PermissionRequest,
    /// A permission was denied.
    PermissionDenied,
    /// Setup ran.
    Setup,
    /// A teammate went idle.
    TeammateIdle,
    /// A task was created.
    TaskCreated,
    /// A task completed.
    TaskCompleted,
    /// An MCP elicitation was requested.
    Elicitation,
    /// An MCP elicitation was answered.
    ElicitationResult,
    /// Configuration changed on disk.
    ConfigChange,
    /// A worktree was created.
    WorktreeCreate,
    /// A worktree was removed.
    WorktreeRemove,
    /// Memory instructions were loaded.
    InstructionsLoaded,
    /// The working directory changed.
    CwdChanged,
    /// A watched file changed.
    FileChanged,
}

/// One hook registration: what it matches, and which callbacks it
/// routes to.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HookCallbackMatcher {
    /// The matcher pattern, when the event takes one.
    pub matcher: Option<String>,
    /// The callbacks to deliver to.
    #[serde(rename = "hookCallbackIds")]
    pub hook_callback_ids: Vec<String>,
    /// The callback timeout, seconds.
    pub timeout: Option<f64>,
}

/// An SDK-defined subagent, as `initialize` carries them — the same
/// vocabulary the `--agents` flag takes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AgentDefinition {
    /// When to use this agent, in words.
    pub description: String,
    /// Allowed tools; absent inherits the parent's.
    pub tools: Option<Vec<String>>,
    /// Tools explicitly denied.
    #[serde(
        rename = "disallowedTools"
    )]
    pub disallowed_tools: Option<Vec<String>>,
    /// The agent's system prompt.
    pub prompt: String,
    /// A model alias or id; absent or `inherit` uses the main model.
    pub model: Option<String>,
    /// MCP servers of the agent's own.
    #[serde(
        rename = "mcpServers"
    )]
    pub mcp_servers: Option<Vec<AgentMcpServerSpec>>,
    /// Experimental: a reminder re-injected into the system prompt.
    #[serde(
        rename = "criticalSystemReminder_EXPERIMENTAL"
    )]
    pub critical_system_reminder: Option<String>,
    /// Skills preloaded into the agent's context.
    pub skills: Option<Vec<String>>,
    /// Auto-submitted as the first user turn when this agent is the
    /// main-thread agent.
    #[serde(
        rename = "initialPrompt"
    )]
    pub initial_prompt: Option<String>,
    /// Turn cap.
    #[serde(
        rename = "maxTurns"
    )]
    pub max_turns: Option<u64>,
    /// Run as a background task when invoked.
    pub background: Option<bool>,
    /// Where the agent's persistent memory lives.
    pub memory: Option<AgentMemoryScope>,
    /// Reasoning effort: a named level or an integer.
    pub effort: Option<Effort>,
    /// The agent's permission mode.
    #[serde(
        rename = "permissionMode"
    )]
    pub permission_mode: Option<PermissionMode>,
}

/// An agent's MCP server: a reference to a configured one by name,
/// or an inline `{name: config}` definition. Untagged, because a
/// string and an object cannot collide.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum AgentMcpServerSpec {
    /// A configured server, by name.
    Name(String),
    /// Inline definitions, name to config.
    Inline(indexmap::IndexMap<String, McpServerConfig>),
}

/// Where an agent's memory auto-loads from.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AgentMemoryScope {
    /// The user's config directory.
    User,
    /// The project's checked-in `.claude`.
    Project,
    /// The project's gitignored `.claude`.
    Local,
}

/// Reasoning effort: a named level or a bare integer — the source's
/// own union, untagged because a string and a number cannot collide.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Effort {
    /// A named level.
    Level(EffortLevel),
    /// An integer level.
    Integer(i64),
}

/// The named effort levels.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum EffortLevel {
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High.
    High,
    /// Max.
    Max,
}

/// An MCP server config, as the process-transport vocabulary defines
/// it: four transports. Untagged rather than `type`-tagged because
/// the stdio variant's `type` is OPTIONAL on the wire — an untagged
/// try-in-order with literal markers on the other three reads
/// exactly what the source's union admits. Stdio is tried last, so
/// the tagged three claim their literals first.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum McpServerConfig {
    /// Streamed over SSE.
    Sse {
        /// Always `sse`.
        r#type: SseType,
        /// The server's URL.
        url: String,
        /// Headers to send it.
        headers: Option<indexmap::IndexMap<String, String>>,
    },
    /// Streamable HTTP.
    Http {
        /// Always `http`.
        r#type: HttpType,
        /// The server's URL.
        url: String,
        /// Headers to send it.
        headers: Option<indexmap::IndexMap<String, String>>,
    },
    /// Hosted by the SDK consumer itself.
    Sdk {
        /// Always `sdk`.
        r#type: SdkType,
        /// The server's name.
        name: String,
    },
    /// A child process on stdio — the `type` is optional for
    /// backwards compatibility, which is why this union is untagged.
    Stdio {
        /// Always `stdio`, when present at all.
        r#type: Option<StdioType>,
        /// The command to spawn.
        command: String,
        /// Its arguments.
        args: Option<Vec<String>>,
        /// Its environment.
        env: Option<indexmap::IndexMap<String, String>>,
    },
}

/// [`McpServerConfig::Sse`]'s literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SseType {
    /// The only value.
    #[default]
    Sse,
}

/// [`McpServerConfig::Http`]'s literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum HttpType {
    /// The only value.
    #[default]
    Http,
}

/// [`McpServerConfig::Sdk`]'s literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SdkType {
    /// The only value.
    #[default]
    Sdk,
}

/// [`McpServerConfig::Stdio`]'s literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StdioType {
    /// The only value.
    #[default]
    Stdio,
}

/// The `addRules` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum AddRulesType {
    /// The only value.
    #[default]
    AddRules,
}

/// The `replaceRules` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum ReplaceRulesType {
    /// The only value.
    #[default]
    ReplaceRules,
}

/// The `removeRules` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum RemoveRulesType {
    /// The only value.
    #[default]
    RemoveRules,
}

/// The `setMode` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum SetModeType {
    /// The only value.
    #[default]
    SetMode,
}

/// The `addDirectories` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum AddDirectoriesType {
    /// The only value.
    #[default]
    AddDirectories,
}

/// The `removeDirectories` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum RemoveDirectoriesType {
    /// The only value.
    #[default]
    RemoveDirectories,
}
