//! The `system` records: everything Claude Code says about itself.

use serde::{Deserialize, Serialize};

use super::AssistantMessageError;

/// A `type: "system"` record, discriminated by `subtype`.
///
/// The run's own narration, as opposed to the conversation: what the
/// session is, what changed, what a hook or a background task did.
/// Every subtype the source can emit is here — including
/// [`LocalCommandOutput`](Self::LocalCommandOutput), which the schemas
/// define and nothing currently sends, and
/// [`BridgeState`](Self::BridgeState), which is sent and never made
/// it into the schemas at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum System {
    /// The turn beginning: the session introduced. Emitted once per
    /// TURN — every `ask()` leads with one — not once per process.
    Init {
        /// The subagent roster, by name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agents: Option<Vec<String>>,
        /// Where the credential came from.
        #[serde(rename = "apiKeySource")]
        api_key_source: ApiKeySource,
        /// API beta flags in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        betas: Option<Vec<String>>,
        /// Claude Code's own version.
        claude_code_version: String,
        /// The working directory.
        cwd: String,
        /// The tool roster, by name.
        tools: Vec<String>,
        /// The MCP servers and their connection status.
        mcp_servers: Vec<McpServerStatus>,
        /// The model being run.
        model: String,
        /// The permission mode in force.
        #[serde(rename = "permissionMode")]
        permission_mode: PermissionMode,
        /// The slash commands on offer.
        slash_commands: Vec<String>,
        /// The output style in force.
        output_style: String,
        /// The skill roster, by name.
        skills: Vec<String>,
        /// The plugins loaded.
        plugins: Vec<Plugin>,
        /// Fast mode's state, when the feature is in play.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fast_mode_state: Option<FastModeState>,
        /// Internal-only: the UDS inbox socket path, present only on
        /// internal builds with that feature.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        messaging_socket_path: Option<String>,
        /// The record's own id.
        uuid: String,
        /// The session, which the continuation is keyed by.
        session_id: String,
    },
    /// The conversation was compacted here.
    CompactBoundary {
        /// What the compaction did.
        compact_metadata: CompactMetadata,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A status change: compaction starting or ending, or the
    /// permission mode changing.
    Status {
        /// `compacting`, or `null` for back-to-normal.
        status: Option<Status>,
        /// The new permission mode, when that is what changed.
        #[serde(
            rename = "permissionMode",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        permission_mode: Option<PermissionMode>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A background summary of the turn that just ended.
    PostTurnSummary {
        /// The assistant message this summarizes, by uuid.
        summarizes_uuid: String,
        /// Where the work stands.
        status_category: PostTurnStatusCategory,
        /// The status, in words.
        status_detail: String,
        /// Whether the summary is worth surfacing.
        is_noteworthy: bool,
        /// A title for the turn.
        title: String,
        /// What happened.
        description: String,
        /// The most recent action taken.
        recent_action: String,
        /// What still needs doing.
        needs_action: String,
        /// Artifacts the turn produced, by URL.
        artifact_urls: Vec<String>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// An API request failed retryably and will be retried.
    ApiRetry {
        /// Which attempt just failed.
        attempt: u64,
        /// How many will be made.
        max_retries: u64,
        /// How long until the next one.
        retry_delay_ms: u64,
        /// The HTTP status, or `null` for a connection error that
        /// never got a response.
        error_status: Option<i64>,
        /// The failure's kind, in the same vocabulary an assistant
        /// record's error uses.
        error: AssistantMessageError,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A local slash command's output. Defined by the schemas and
    /// currently sent by nothing — the source ships it as a synthetic
    /// assistant message instead — but a defined shape is a shape a
    /// later version may use.
    LocalCommandOutput {
        /// The output text.
        content: String,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A hook began.
    HookStarted {
        /// The execution's id, shared by its progress and response.
        hook_id: String,
        /// The hook's name.
        hook_name: String,
        /// Which lifecycle event fired it.
        hook_event: String,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A hook produced output mid-run.
    HookProgress {
        /// The execution's id.
        hook_id: String,
        /// The hook's name.
        hook_name: String,
        /// Which lifecycle event fired it.
        hook_event: String,
        /// Standard output so far.
        stdout: String,
        /// Standard error so far.
        stderr: String,
        /// The combined output so far.
        output: String,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A hook finished.
    HookResponse {
        /// The execution's id.
        hook_id: String,
        /// The hook's name.
        hook_name: String,
        /// Which lifecycle event fired it.
        hook_event: String,
        /// The combined output.
        output: String,
        /// Standard output.
        stdout: String,
        /// Standard error.
        stderr: String,
        /// The exit code, when the hook ran far enough to have one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i64>,
        /// How it ended.
        outcome: HookOutcome,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A background task began.
    TaskStarted {
        /// The task's id.
        task_id: String,
        /// The tool call that spawned it, if one did.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
        /// What the task is.
        description: String,
        /// The task's kind.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_type: Option<String>,
        /// The workflow's name, for workflow tasks.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workflow_name: Option<String>,
        /// The prompt it was given.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A background task moved.
    TaskProgress {
        /// The task's id.
        task_id: String,
        /// The tool call that spawned it, if one did.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
        /// What the task is.
        description: String,
        /// What it has spent.
        usage: TaskUsage,
        /// The last tool it ran.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_tool_name: Option<String>,
        /// A summary of where it stands.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        /// Workflow state deltas. Sent by the source but absent from
        /// its schemas, and its element type lives in a file the
        /// clone does not carry — so the elements stay unread.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workflow_progress: Option<Vec<serde_json::Value>>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A background task ended.
    TaskNotification {
        /// The task's id.
        task_id: String,
        /// The tool call that spawned it, if one did.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
        /// How it ended.
        status: TaskStatus,
        /// Where its output landed.
        output_file: String,
        /// What it did, in words.
        summary: String,
        /// What it spent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<TaskUsage>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// The session's state moved. `idle` is the AUTHORITATIVE
    /// turn-over signal: a `result` can be held back past later
    /// records while background agents run, but `idle` fires only
    /// once everything has flushed.
    SessionStateChanged {
        /// The new state.
        state: SessionState,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// Files were persisted upstream, on builds with that feature.
    FilesPersisted {
        /// What made it.
        files: Vec<PersistedFile>,
        /// What did not.
        failed: Vec<FailedFile>,
        /// When the batch was processed, ISO 8601.
        processed_at: String,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// An MCP server confirmed a URL-mode elicitation finished.
    ElicitationComplete {
        /// The server that confirmed it.
        mcp_server_name: String,
        /// Which elicitation.
        elicitation_id: String,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// The remote-control bridge changed state. Emitted by the
    /// source through an escape-hatch cast, declared in no schema —
    /// typed here because it is real.
    BridgeState {
        /// The bridge's new state.
        state: BridgeState,
        /// Detail, when the state has any — a failure's reason.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
}

/// One MCP server's connection status in an
/// [`Init`](System::Init) record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct McpServerStatus {
    /// The server's configured name.
    pub name: String,
    /// Its connection state, as the source words it.
    pub status: String,
}

/// One loaded plugin in an [`Init`](System::Init) record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Plugin {
    /// The plugin's name.
    pub name: String,
    /// Where it lives on disk.
    pub path: String,
    /// Its source identifier, `name@marketplace` — with `name@inline`
    /// for `--plugin-dir` plugins and `name@builtin` for built-ins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Where the API credential came from.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeySource {
    /// The user's own key.
    User,
    /// A project key.
    Project,
    /// An organization key.
    Org,
    /// A temporary key.
    Temporary,
    /// An OAuth login.
    Oauth,
}

/// The permission mode in force.
///
/// The schema's five, plus [`Auto`](Self::Auto): the source's
/// permission-mode-change emitter can say `auto` even though its own
/// schema does not admit it, and a reader that refused the value
/// would be stricter than the writer.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum PermissionMode {
    /// Standard behavior: prompts for dangerous operations.
    #[serde(rename = "default")]
    Default,
    /// File edits auto-accepted.
    #[serde(rename = "acceptEdits")]
    AcceptEdits,
    /// Everything allowed, nothing asked.
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
    /// Plan mode: read-only until a plan is approved.
    #[serde(rename = "plan")]
    Plan,
    /// Never prompt; denied is denied.
    #[serde(rename = "dontAsk")]
    DontAsk,
    /// The schema-escaping sixth, see the type doc.
    #[serde(rename = "auto")]
    Auto,
}

/// Fast mode's state.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FastModeState {
    /// Off.
    Off,
    /// Cooling down after a rate limit.
    Cooldown,
    /// On.
    On,
}

/// What a [`CompactBoundary`](System::CompactBoundary) did.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompactMetadata {
    /// Who asked for it.
    pub trigger: CompactTrigger,
    /// How many tokens the conversation held before.
    pub pre_tokens: u64,
    /// Relink info for a partial compact that preserved a segment;
    /// unset when everything was summarized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserved_segment: Option<PreservedSegment>,
}

/// Who asked for a compaction.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CompactTrigger {
    /// The user did.
    Manual,
    /// The context ceiling did.
    Auto,
}

/// The segment a partial compact preserved, by the uuids a loader
/// splices at.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PreservedSegment {
    /// The segment's first message.
    pub head_uuid: String,
    /// Where the splice anchors.
    pub anchor_uuid: String,
    /// The segment's last message.
    pub tail_uuid: String,
}

/// The one non-null status a [`Status`](System::Status) record can
/// carry.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// The conversation is being compacted.
    Compacting,
}

/// Where a [`PostTurnSummary`](System::PostTurnSummary) puts the
/// work.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PostTurnStatusCategory {
    /// Stuck on something.
    Blocked,
    /// Waiting on something.
    Waiting,
    /// Done.
    Completed,
    /// Ready for review.
    ReviewReady,
    /// Failed.
    Failed,
}

/// How a hook ended.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum HookOutcome {
    /// It ran and succeeded.
    Success,
    /// It ran and failed.
    Error,
    /// It was cancelled.
    Cancelled,
}

/// What a background task has spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskUsage {
    /// Tokens, all kinds.
    pub total_tokens: u64,
    /// Tool calls made.
    pub tool_uses: u64,
    /// Wall-clock spent.
    pub duration_ms: u64,
}

/// How a background task ended.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// It finished.
    Completed,
    /// It failed.
    Failed,
    /// It was stopped.
    Stopped,
}

/// The session's state.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Nothing running — the authoritative turn-over signal.
    Idle,
    /// A turn in flight.
    Running,
    /// Waiting on the user.
    RequiresAction,
}

/// One file a [`FilesPersisted`](System::FilesPersisted) batch
/// landed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PersistedFile {
    /// The file's name.
    pub filename: String,
    /// The id it landed under.
    pub file_id: String,
}

/// One file a [`FilesPersisted`](System::FilesPersisted) batch lost.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FailedFile {
    /// The file's name.
    pub filename: String,
    /// Why it did not land.
    pub error: String,
}

/// The remote-control bridge's states.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BridgeState {
    /// Up, before the first connection.
    Ready,
    /// Connected.
    Connected,
    /// Connection lost, trying again.
    Reconnecting,
    /// Given up.
    Failed,
}
