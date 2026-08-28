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

use serde::{Deserialize, Serialize};

use super::system::PermissionMode;
use config::{
    AgentDefinition, HookCallbackMatcher, HookEvent, McpServerConfig,
    PermissionUpdate,
};
use hook_input::HookInput;

/// A `type: "control_request"` record: one ask, to be answered by a
/// `control_response` quoting its [`request_id`](Self::request_id).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlRequest {
    /// The ask's id, quoted by its answer.
    pub request_id: String,
    /// The ask itself.
    pub request: ControlRequestInner,
}

/// What a control request asks, discriminated by `subtype` — the
/// source's twenty-one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlRequestInner {
    /// Stop the running turn.
    Interrupt,
    /// May this tool run? The one subtype plain print mode really
    /// sends outward — sandbox network asks ride it too.
    CanUseTool {
        /// The tool.
        tool_name: String,
        /// What it would run with.
        input: indexmap::IndexMap<String, serde_json::Value>,
        /// Permission rules that would allow it, ready to apply.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_suggestions: Option<Vec<PermissionUpdate>>,
        /// The path a rule blocked, when one did.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blocked_path: Option<String>,
        /// Why the decision is being asked.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decision_reason: Option<String>,
        /// A title for the prompt.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// A display name for the tool.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        /// The call awaiting the verdict.
        tool_use_id: String,
        /// The agent asking, when a subagent is.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// What the call would do, in words.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// Configure the SDK session.
    Initialize {
        /// Hook registrations by event.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hooks: Option<
            indexmap::IndexMap<HookEvent, Vec<HookCallbackMatcher>>,
        >,
        /// SDK-hosted MCP servers, by name.
        #[serde(
            rename = "sdkMcpServers",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        sdk_mcp_servers: Option<Vec<String>>,
        /// A JSON schema for structured output.
        #[serde(
            rename = "jsonSchema",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        json_schema: Option<indexmap::IndexMap<String, serde_json::Value>>,
        /// A replacement system prompt.
        #[serde(
            rename = "systemPrompt",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        system_prompt: Option<String>,
        /// An addition to the system prompt.
        #[serde(
            rename = "appendSystemPrompt",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        append_system_prompt: Option<String>,
        /// Agent definitions by name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agents: Option<indexmap::IndexMap<String, AgentDefinition>>,
        /// Whether to emit prompt suggestions.
        #[serde(
            rename = "promptSuggestions",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        prompt_suggestions: Option<bool>,
        /// Whether to emit agent progress summaries.
        #[serde(
            rename = "agentProgressSummaries",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        agent_progress_summaries: Option<bool>,
    },
    /// Change the permission mode.
    SetPermissionMode {
        /// The new mode.
        mode: PermissionMode,
        /// Internal remote-session marker.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ultraplan: Option<bool>,
    },
    /// Change the model.
    SetModel {
        /// The new model; absent means the default.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    /// Change the thinking budget.
    SetMaxThinkingTokens {
        /// The new budget; `null` clears it.
        max_thinking_tokens: Option<u64>,
    },
    /// Ask after the MCP servers.
    McpStatus,
    /// Ask for the context-window breakdown.
    GetContextUsage,
    /// Deliver a hook callback's input.
    HookCallback {
        /// Which registered callback.
        callback_id: String,
        /// The hook's input, typed per lifecycle event.
        input: HookInput,
        /// The tool call involved, when one is.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
    },
    /// Relay a JSON-RPC message to an MCP server.
    McpMessage {
        /// The server.
        server_name: String,
        /// The message — `unknown` in the source's own schema, and
        /// optional the way every bare `z.unknown()` is: the key may
        /// be absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<serde_json::Value>,
    },
    /// Rewind file changes to a user message.
    RewindFiles {
        /// The message to rewind to.
        user_message_id: String,
        /// Whether to only report what would change.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dry_run: Option<bool>,
    },
    /// Drop a queued async message.
    CancelAsyncMessage {
        /// The queued message, by uuid.
        message_uuid: String,
    },
    /// Seed the read-state cache so a later edit validates.
    SeedReadState {
        /// The file.
        path: String,
        /// The mtime the client observed.
        mtime: f64,
    },
    /// Replace the dynamically managed MCP servers.
    McpSetServers {
        /// The new set, name to config.
        servers: indexmap::IndexMap<String, McpServerConfig>,
    },
    /// Reload plugins from disk.
    ReloadPlugins,
    /// Reconnect a failed MCP server.
    McpReconnect {
        /// The server.
        #[serde(rename = "serverName")]
        server_name: String,
    },
    /// Enable or disable an MCP server.
    McpToggle {
        /// The server.
        #[serde(rename = "serverName")]
        server_name: String,
        /// The new state.
        enabled: bool,
    },
    /// Stop a background task.
    StopTask {
        /// The task.
        task_id: String,
    },
    /// Merge settings into the flag layer.
    ApplyFlagSettings {
        /// The settings to merge.
        settings: indexmap::IndexMap<String, serde_json::Value>,
    },
    /// Ask for the effective settings.
    GetSettings,
    /// Ask the SDK consumer to run an MCP elicitation.
    Elicitation {
        /// The server eliciting.
        mcp_server_name: String,
        /// What it wants to say.
        message: String,
        /// Form or URL mode.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<ElicitationMode>,
        /// The URL, in URL mode.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        /// The elicitation's id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        elicitation_id: Option<String>,
        /// The schema the answer should satisfy.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        requested_schema:
            Option<indexmap::IndexMap<String, serde_json::Value>>,
    },
}

/// An elicitation's mode.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlResponse {
    /// The verdict.
    pub response: ControlResponseInner,
}

/// A control response's body, discriminated by `subtype`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlResponseInner {
    /// The request succeeded.
    Success {
        /// The request being answered.
        request_id: String,
        /// The response body, shaped per request subtype — a record
        /// the source types per subtype and wraps as a plain map.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        response: Option<indexmap::IndexMap<String, serde_json::Value>>,
    },
    /// The request failed.
    Error {
        /// The request being answered.
        request_id: String,
        /// What went wrong.
        error: String,
        /// Permission asks still open when the failure happened.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pending_permission_requests: Option<Vec<ControlRequest>>,
    },
}

/// A `type: "control_cancel_request"` record: an open control
/// request, withdrawn.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlCancelRequest {
    /// The ask being withdrawn.
    pub request_id: String,
}
