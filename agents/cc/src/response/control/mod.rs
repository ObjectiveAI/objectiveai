//! The control-protocol records: the SDK conversation about the
//! conversation.
//!
//! Control requests normally travel INTO Claude Code on stdin, from
//! an SDK driving it; stdout carries them in two narrow cases — a
//! sandbox network ask, and `--permission-prompt-tool stdio` — plus
//! responses to whatever arrived on stdin, and cancellations. All of
//! it is in the stdout union, so all of it is here.
//!
//! # Typed all the way down
//!
//! Each of the twenty-one request subtypes carries its own fields,
//! and the configuration vocabularies those fields open onto are
//! typed too: [`hook_input`]'s twenty-seven event inputs, and
//! [`config`]'s permission updates, hook registrations, agent
//! definitions, and MCP server configs. What remains
//! [`serde_json::Value`] is only what the source itself types
//! `unknown`: tool inputs (the tool's schema, not the stream's),
//! JSON schemas, settings blobs, the JSON-RPC message of
//! `mcp_message`, and the per-request `control_response` body, whose
//! shape depends on request correlation no line-at-a-time reader
//! has.

pub mod config;
pub mod hook_input;

use serde::Deserialize;

use super::system::PermissionMode;
use config::{
    AgentDefinition, HookCallbackMatcher, HookEvent, McpServerConfig,
    PermissionUpdate,
};
use hook_input::HookInput;

/// A `type: "control_request"` record: one ask, to be answered by a
/// `control_response` quoting its [`request_id`](Self::request_id).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ControlRequest {
    /// Always `control_request`.
    pub r#type: ControlRequestType,
    /// The ask's id, quoted by its answer.
    pub request_id: String,
    /// The ask itself.
    pub request: ControlRequestInner,
}

/// What a control request asks — the source's twenty-one subtypes,
/// untagged, each variant carrying its `subtype` literal as a marker
/// field so the object is self-describing wherever it travels.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ControlRequestInner {
    /// Stop the running turn.
    Interrupt {
        /// Always `interrupt`.
        subtype: InterruptSubtype,
    },
    /// May this tool run? The one subtype plain print mode really
    /// sends outward — sandbox network asks ride it too.
    CanUseTool {
        /// Always `can_use_tool`.
        subtype: CanUseToolSubtype,
        /// The tool.
        tool_name: String,
        /// What it would run with.
        input: indexmap::IndexMap<String, serde_json::Value>,
        /// Permission rules that would allow it, ready to apply.
        permission_suggestions: Option<Vec<PermissionUpdate>>,
        /// The path a rule blocked, when one did.
        blocked_path: Option<String>,
        /// Why the decision is being asked.
        decision_reason: Option<String>,
        /// A title for the prompt.
        title: Option<String>,
        /// A display name for the tool.
        display_name: Option<String>,
        /// The call awaiting the verdict.
        tool_use_id: String,
        /// The agent asking, when a subagent is.
        agent_id: Option<String>,
        /// What the call would do, in words.
        description: Option<String>,
    },
    /// Configure the SDK session.
    Initialize {
        /// Always `initialize`.
        subtype: InitializeSubtype,
        /// Hook registrations by event.
        hooks: Option<
            indexmap::IndexMap<HookEvent, Vec<HookCallbackMatcher>>,
        >,
        /// SDK-hosted MCP servers, by name.
        #[serde(
            rename = "sdkMcpServers"
        )]
        sdk_mcp_servers: Option<Vec<String>>,
        /// A JSON schema for structured output.
        #[serde(
            rename = "jsonSchema"
        )]
        json_schema: Option<indexmap::IndexMap<String, serde_json::Value>>,
        /// A replacement system prompt.
        #[serde(
            rename = "systemPrompt"
        )]
        system_prompt: Option<String>,
        /// An addition to the system prompt.
        #[serde(
            rename = "appendSystemPrompt"
        )]
        append_system_prompt: Option<String>,
        /// Agent definitions by name.
        agents: Option<indexmap::IndexMap<String, AgentDefinition>>,
        /// Whether to emit prompt suggestions.
        #[serde(
            rename = "promptSuggestions"
        )]
        prompt_suggestions: Option<bool>,
        /// Whether to emit agent progress summaries.
        #[serde(
            rename = "agentProgressSummaries"
        )]
        agent_progress_summaries: Option<bool>,
    },
    /// Change the permission mode.
    SetPermissionMode {
        /// Always `set_permission_mode`.
        subtype: SetPermissionModeSubtype,
        /// The new mode.
        mode: PermissionMode,
        /// Internal remote-session marker.
        ultraplan: Option<bool>,
    },
    /// Change the model.
    SetModel {
        /// Always `set_model`.
        subtype: SetModelSubtype,
        /// The new model; absent means the default.
        model: Option<String>,
    },
    /// Change the thinking budget.
    SetMaxThinkingTokens {
        /// Always `set_max_thinking_tokens`.
        subtype: SetMaxThinkingTokensSubtype,
        /// The new budget; `null` clears it.
        max_thinking_tokens: Option<u64>,
    },
    /// Ask after the MCP servers.
    McpStatus {
        /// Always `mcp_status`.
        subtype: McpStatusSubtype,
    },
    /// Ask for the context-window breakdown.
    GetContextUsage {
        /// Always `get_context_usage`.
        subtype: GetContextUsageSubtype,
    },
    /// Deliver a hook callback's input.
    HookCallback {
        /// Always `hook_callback`.
        subtype: HookCallbackSubtype,
        /// Which registered callback.
        callback_id: String,
        /// The hook's input, typed per lifecycle event.
        input: HookInput,
        /// The tool call involved, when one is.
        tool_use_id: Option<String>,
    },
    /// Relay a JSON-RPC message to an MCP server.
    McpMessage {
        /// Always `mcp_message`.
        subtype: McpMessageSubtype,
        /// The server.
        server_name: String,
        /// The message — `unknown` in the source's own schema, and
        /// optional the way every bare `z.unknown()` is: the key may
        /// be absent.
        message: Option<serde_json::Value>,
    },
    /// Rewind file changes to a user message.
    RewindFiles {
        /// Always `rewind_files`.
        subtype: RewindFilesSubtype,
        /// The message to rewind to.
        user_message_id: String,
        /// Whether to only report what would change.
        dry_run: Option<bool>,
    },
    /// Drop a queued async message.
    CancelAsyncMessage {
        /// Always `cancel_async_message`.
        subtype: CancelAsyncMessageSubtype,
        /// The queued message, by uuid.
        message_uuid: String,
    },
    /// Seed the read-state cache so a later edit validates.
    SeedReadState {
        /// Always `seed_read_state`.
        subtype: SeedReadStateSubtype,
        /// The file.
        path: String,
        /// The mtime the client observed.
        mtime: f64,
    },
    /// Replace the dynamically managed MCP servers.
    McpSetServers {
        /// Always `mcp_set_servers`.
        subtype: McpSetServersSubtype,
        /// The new set, name to config.
        servers: indexmap::IndexMap<String, McpServerConfig>,
    },
    /// Reload plugins from disk.
    ReloadPlugins {
        /// Always `reload_plugins`.
        subtype: ReloadPluginsSubtype,
    },
    /// Reconnect a failed MCP server.
    McpReconnect {
        /// Always `mcp_reconnect`.
        subtype: McpReconnectSubtype,
        /// The server.
        #[serde(rename = "serverName")]
        server_name: String,
    },
    /// Enable or disable an MCP server.
    McpToggle {
        /// Always `mcp_toggle`.
        subtype: McpToggleSubtype,
        /// The server.
        #[serde(rename = "serverName")]
        server_name: String,
        /// The new state.
        enabled: bool,
    },
    /// Stop a background task.
    StopTask {
        /// Always `stop_task`.
        subtype: StopTaskSubtype,
        /// The task.
        task_id: String,
    },
    /// Merge settings into the flag layer.
    ApplyFlagSettings {
        /// Always `apply_flag_settings`.
        subtype: ApplyFlagSettingsSubtype,
        /// The settings to merge.
        settings: indexmap::IndexMap<String, serde_json::Value>,
    },
    /// Ask for the effective settings.
    GetSettings {
        /// Always `get_settings`.
        subtype: GetSettingsSubtype,
    },
    /// Ask the SDK consumer to run an MCP elicitation.
    Elicitation {
        /// Always `elicitation`.
        subtype: ElicitationSubtype,
        /// The server eliciting.
        mcp_server_name: String,
        /// What it wants to say.
        message: String,
        /// Form or URL mode.
        mode: Option<ElicitationMode>,
        /// The URL, in URL mode.
        url: Option<String>,
        /// The elicitation's id.
        elicitation_id: Option<String>,
        /// The schema the answer should satisfy.
        requested_schema:
            Option<indexmap::IndexMap<String, serde_json::Value>>,
    },
}

/// An elicitation's mode.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ElicitationMode {
    /// A form the consumer renders.
    Form,
    /// A URL the consumer opens.
    Url,
}

/// A `type: "control_response"` record: the answer to a control
/// request, either way.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ControlResponse {
    /// Always `control_response`.
    pub r#type: ControlResponseType,
    /// The verdict.
    pub response: ControlResponseInner,
}

/// A control response's body — two arms, untagged, each carrying
/// its `subtype` literal as a marker field.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ControlResponseInner {
    /// The request succeeded.
    Success {
        /// Always `success`.
        subtype: SuccessSubtype,
        /// The request being answered.
        request_id: String,
        /// The response body, shaped per request subtype — a record
        /// the source types per subtype and wraps as a plain map.
        response: Option<indexmap::IndexMap<String, serde_json::Value>>,
    },
    /// The request failed.
    Error {
        /// Always `error`.
        subtype: ErrorSubtype,
        /// The request being answered.
        request_id: String,
        /// What went wrong.
        error: String,
        /// Permission asks still open when the failure happened.
        pending_permission_requests: Option<Vec<ControlRequest>>,
    },
}

/// A `type: "control_cancel_request"` record: an open control
/// request, withdrawn.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ControlCancelRequest {
    /// Always `control_cancel_request`.
    pub r#type: ControlCancelRequestType,
    /// The ask being withdrawn.
    pub request_id: String,
}

/// The `control_request` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ControlRequestType {
    /// The only value.
    #[default]
    ControlRequest,
}

/// The `control_response` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ControlResponseType {
    /// The only value.
    #[default]
    ControlResponse,
}

/// The `control_cancel_request` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ControlCancelRequestType {
    /// The only value.
    #[default]
    ControlCancelRequest,
}

/// The `can_use_tool` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CanUseToolSubtype {
    /// The only value.
    #[default]
    CanUseTool,
}

/// The `initialize` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum InitializeSubtype {
    /// The only value.
    #[default]
    Initialize,
}

/// The `set_permission_mode` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SetPermissionModeSubtype {
    /// The only value.
    #[default]
    SetPermissionMode,
}

/// The `set_model` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SetModelSubtype {
    /// The only value.
    #[default]
    SetModel,
}

/// The `set_max_thinking_tokens` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SetMaxThinkingTokensSubtype {
    /// The only value.
    #[default]
    SetMaxThinkingTokens,
}

/// The `hook_callback` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum HookCallbackSubtype {
    /// The only value.
    #[default]
    HookCallback,
}

/// The `mcp_message` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum McpMessageSubtype {
    /// The only value.
    #[default]
    McpMessage,
}

/// The `rewind_files` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RewindFilesSubtype {
    /// The only value.
    #[default]
    RewindFiles,
}

/// The `cancel_async_message` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CancelAsyncMessageSubtype {
    /// The only value.
    #[default]
    CancelAsyncMessage,
}

/// The `seed_read_state` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SeedReadStateSubtype {
    /// The only value.
    #[default]
    SeedReadState,
}

/// The `mcp_set_servers` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum McpSetServersSubtype {
    /// The only value.
    #[default]
    McpSetServers,
}

/// The `mcp_reconnect` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum McpReconnectSubtype {
    /// The only value.
    #[default]
    McpReconnect,
}

/// The `mcp_toggle` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum McpToggleSubtype {
    /// The only value.
    #[default]
    McpToggle,
}

/// The `stop_task` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StopTaskSubtype {
    /// The only value.
    #[default]
    StopTask,
}

/// The `apply_flag_settings` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ApplyFlagSettingsSubtype {
    /// The only value.
    #[default]
    ApplyFlagSettings,
}

/// The `elicitation` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ElicitationSubtype {
    /// The only value.
    #[default]
    Elicitation,
}

/// The `interrupt` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum InterruptSubtype {
    /// The only value.
    #[default]
    Interrupt,
}

/// The `mcp_status` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum McpStatusSubtype {
    /// The only value.
    #[default]
    McpStatus,
}

/// The `get_context_usage` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum GetContextUsageSubtype {
    /// The only value.
    #[default]
    GetContextUsage,
}

/// The `reload_plugins` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReloadPluginsSubtype {
    /// The only value.
    #[default]
    ReloadPlugins,
}

/// The `get_settings` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum GetSettingsSubtype {
    /// The only value.
    #[default]
    GetSettings,
}

/// The `success` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SuccessSubtype {
    /// The only value.
    #[default]
    Success,
}

/// The `error` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSubtype {
    /// The only value.
    #[default]
    Error,
}
