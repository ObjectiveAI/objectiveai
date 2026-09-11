//! The `system` records: everything Claude Code says about itself.

use serde::Deserialize;

use super::assistant::AssistantMessageError;

/// A `type: "system"` record: one of sixteen subtypes, each
/// self-describing — untagged, with the `type` and `subtype`
/// literals as marker fields on every variant.
///
/// The run's own narration, as opposed to the conversation: what the
/// session is, what changed, what a hook or a background task did.
/// Every subtype the source can emit is here — including
/// [`LocalCommandOutput`](Self::LocalCommandOutput), which the schemas
/// define and nothing currently sends, and
/// [`BridgeState`](Self::BridgeState), which is sent and never made
/// it into the schemas at all.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum System {
    /// The turn beginning: the session introduced. Emitted once per
    /// TURN — every `ask()` leads with one — not once per process.
    Init {
        /// Always `system`.
        r#type: SystemType,
        /// Always `init`.
        subtype: InitSubtype,
        /// The subagent roster, by name.
        agents: Option<Vec<String>>,
        /// Where the credential came from.
        #[serde(rename = "apiKeySource")]
        api_key_source: ApiKeySource,
        /// API beta flags in force.
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
        fast_mode_state: Option<FastModeState>,
        /// Internal-only: the UDS inbox socket path, present only on
        /// internal builds with that feature.
        messaging_socket_path: Option<String>,
        /// The record's own id.
        uuid: String,
        /// The session, which the continuation is keyed by.
        session_id: String,
    },
    /// The conversation was compacted here.
    CompactBoundary {
        /// Always `system`.
        r#type: SystemType,
        /// Always `compact_boundary`.
        subtype: CompactBoundarySubtype,
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
        /// Always `system`.
        r#type: SystemType,
        /// Always `status`.
        subtype: StatusSubtype,
        /// `compacting`, or `null` for back-to-normal.
        status: Option<Status>,
        /// The new permission mode, when that is what changed.
        #[serde(
            rename = "permissionMode"
        )]
        permission_mode: Option<PermissionMode>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A background summary of the turn that just ended.
    PostTurnSummary {
        /// Always `system`.
        r#type: SystemType,
        /// Always `post_turn_summary`.
        subtype: PostTurnSummarySubtype,
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
        /// Always `system`.
        r#type: SystemType,
        /// Always `api_retry`.
        subtype: ApiRetrySubtype,
        /// Which attempt just failed.
        attempt: u64,
        /// How many will be made.
        max_retries: u64,
        /// How long until the next one. Fractional, because the
        /// source's backoff is jittered by `Math.random()` and only
        /// a `Retry-After` header ever produces a whole number.
        retry_delay_ms: f64,
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
        /// Always `system`.
        r#type: SystemType,
        /// Always `local_command_output`.
        subtype: LocalCommandOutputSubtype,
        /// The output text.
        content: String,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A hook began.
    HookStarted {
        /// Always `system`.
        r#type: SystemType,
        /// Always `hook_started`.
        subtype: HookStartedSubtype,
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
        /// Always `system`.
        r#type: SystemType,
        /// Always `hook_progress`.
        subtype: HookProgressSubtype,
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
        /// Always `system`.
        r#type: SystemType,
        /// Always `hook_response`.
        subtype: HookResponseSubtype,
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
        /// Always `system`.
        r#type: SystemType,
        /// Always `task_started`.
        subtype: TaskStartedSubtype,
        /// The task's id.
        task_id: String,
        /// The tool call that spawned it, if one did.
        tool_use_id: Option<String>,
        /// What the task is.
        description: String,
        /// The task's kind.
        task_type: Option<String>,
        /// The workflow's name, for workflow tasks.
        workflow_name: Option<String>,
        /// The prompt it was given.
        prompt: Option<String>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A background task moved.
    TaskProgress {
        /// Always `system`.
        r#type: SystemType,
        /// Always `task_progress`.
        subtype: TaskProgressSubtype,
        /// The task's id.
        task_id: String,
        /// The tool call that spawned it, if one did.
        tool_use_id: Option<String>,
        /// What the task is.
        description: String,
        /// What it has spent.
        usage: TaskUsage,
        /// The last tool it ran.
        last_tool_name: Option<String>,
        /// A summary of where it stands.
        summary: Option<String>,
        /// Workflow state deltas. Sent by the source but absent from
        /// its schemas, and its element type lives in a file the
        /// clone does not carry — so the elements stay unread.
        workflow_progress: Option<Vec<serde_json::Value>>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// A background task ended.
    TaskNotification {
        /// Always `system`.
        r#type: SystemType,
        /// Always `task_notification`.
        subtype: TaskNotificationSubtype,
        /// The task's id.
        task_id: String,
        /// The tool call that spawned it, if one did.
        tool_use_id: Option<String>,
        /// How it ended.
        status: TaskStatus,
        /// Where its output landed.
        output_file: String,
        /// What it did, in words.
        summary: String,
        /// What it spent.
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
        /// Always `system`.
        r#type: SystemType,
        /// Always `session_state_changed`.
        subtype: SessionStateChangedSubtype,
        /// The new state.
        state: SessionState,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
    /// Files were persisted upstream, on builds with that feature.
    FilesPersisted {
        /// Always `system`.
        r#type: SystemType,
        /// Always `files_persisted`.
        subtype: FilesPersistedSubtype,
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
        /// Always `system`.
        r#type: SystemType,
        /// Always `elicitation_complete`.
        subtype: ElicitationCompleteSubtype,
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
        /// Always `system`.
        r#type: SystemType,
        /// Always `bridge_state`.
        subtype: BridgeStateSubtype,
        /// The bridge's new state.
        state: BridgeState,
        /// Detail, when the state has any — a failure's reason.
        detail: Option<String>,
        /// The record's own id.
        uuid: String,
        /// The session.
        session_id: String,
    },
}

/// One MCP server's connection status in an
/// [`Init`](System::Init) record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct McpServerStatus {
    /// The server's configured name.
    pub name: String,
    /// Its connection state, as the source words it.
    pub status: String,
}

/// One loaded plugin in an [`Init`](System::Init) record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct Plugin {
    /// The plugin's name.
    pub name: String,
    /// Where it lives on disk.
    pub path: String,
    /// Its source identifier, `name@marketplace` — with `name@inline`
    /// for `--plugin-dir` plugins and `name@builtin` for built-ins.
    pub source: Option<String>,
}

/// Where the API credential came from.
///
/// A union of two vocabularies, because the source keeps two and
/// they disagree: the zod schema declares five values, but the only
/// producer writes `getAnthropicApiKeyWithSource().source` through
/// an unchecked cast, and THAT function's return type is a different
/// four. The wire carries the four today; the five stay admitted in
/// case a later version fixes the producer to match its schema.
/// Either way a record deserializes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
pub enum ApiKeySource {
    // The zod schema's five.
    /// The user's own key.
    #[serde(rename = "user")]
    User,
    /// A project key.
    #[serde(rename = "project")]
    Project,
    /// An organization key.
    #[serde(rename = "org")]
    Org,
    /// A temporary key.
    #[serde(rename = "temporary")]
    Temporary,
    /// An OAuth login.
    #[serde(rename = "oauth")]
    Oauth,
    // The producer's four.
    /// The `ANTHROPIC_API_KEY` environment variable.
    #[serde(rename = "ANTHROPIC_API_KEY")]
    AnthropicApiKey,
    /// The configured `apiKeyHelper` command.
    #[serde(rename = "apiKeyHelper")]
    ApiKeyHelper,
    /// A key managed by `/login`.
    #[serde(rename = "/login managed key")]
    LoginManagedKey,
    /// No API key at all — OAuth sessions land here.
    #[serde(rename = "none")]
    None,
}

/// The permission mode in force.
///
/// The schema's five, plus the internal union's two: the source's
/// own `InternalPermissionMode` is the schema's set widened with
/// `auto` and `bubble`, its permission-mode-change emitter can say
/// `auto`, and the `system/init` builder casts the internal mode
/// through unguarded — so a reader that refused either value would
/// be stricter than the writer. `bubble`'s reachability to stdout
/// was never demonstrated, but the cast admits it and refusing it
/// buys nothing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
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
    /// The internal union's seventh, latent — see the type doc.
    #[serde(rename = "bubble")]
    Bubble,
}

/// Fast mode's state.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct CompactMetadata {
    /// Who asked for it.
    pub trigger: CompactTrigger,
    /// How many tokens the conversation held before.
    pub pre_tokens: u64,
    /// Relink info for a partial compact that preserved a segment;
    /// unset when everything was summarized.
    pub preserved_segment: Option<PreservedSegment>,
}

/// Who asked for a compaction.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// The conversation is being compacted.
    Compacting,
}

/// Where a [`PostTurnSummary`](System::PostTurnSummary) puts the
/// work.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct PersistedFile {
    /// The file's name.
    pub filename: String,
    /// The id it landed under.
    pub file_id: String,
}

/// One file a [`FilesPersisted`](System::FilesPersisted) batch lost.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct FailedFile {
    /// The file's name.
    pub filename: String,
    /// Why it did not land.
    pub error: String,
}

/// The remote-control bridge's states.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
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

/// The `system` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SystemType {
    /// The only value.
    #[default]
    System,
}

/// The `init` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum InitSubtype {
    /// The only value.
    #[default]
    Init,
}

/// The `compact_boundary` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CompactBoundarySubtype {
    /// The only value.
    #[default]
    CompactBoundary,
}

/// The `status` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StatusSubtype {
    /// The only value.
    #[default]
    Status,
}

/// The `post_turn_summary` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PostTurnSummarySubtype {
    /// The only value.
    #[default]
    PostTurnSummary,
}

/// The `api_retry` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ApiRetrySubtype {
    /// The only value.
    #[default]
    ApiRetry,
}

/// The `local_command_output` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LocalCommandOutputSubtype {
    /// The only value.
    #[default]
    LocalCommandOutput,
}

/// The `hook_started` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum HookStartedSubtype {
    /// The only value.
    #[default]
    HookStarted,
}

/// The `hook_progress` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum HookProgressSubtype {
    /// The only value.
    #[default]
    HookProgress,
}

/// The `hook_response` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum HookResponseSubtype {
    /// The only value.
    #[default]
    HookResponse,
}

/// The `task_started` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskStartedSubtype {
    /// The only value.
    #[default]
    TaskStarted,
}

/// The `task_progress` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskProgressSubtype {
    /// The only value.
    #[default]
    TaskProgress,
}

/// The `task_notification` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskNotificationSubtype {
    /// The only value.
    #[default]
    TaskNotification,
}

/// The `session_state_changed` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SessionStateChangedSubtype {
    /// The only value.
    #[default]
    SessionStateChanged,
}

/// The `files_persisted` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FilesPersistedSubtype {
    /// The only value.
    #[default]
    FilesPersisted,
}

/// The `elicitation_complete` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ElicitationCompleteSubtype {
    /// The only value.
    #[default]
    ElicitationComplete,
}

/// The `bridge_state` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BridgeStateSubtype {
    /// The only value.
    #[default]
    BridgeState,
}
