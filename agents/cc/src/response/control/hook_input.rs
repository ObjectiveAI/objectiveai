//! What a hook callback is handed: one input per lifecycle event.

use serde::Deserialize;

use super::super::assistant::AssistantMessageError;
use super::config::PermissionUpdate;

/// A hook's input — the source's twenty-seven, each the base fields
/// intersected with the event's own. Untagged, each variant carrying
/// its `hook_event_name` literal (PascalCase, as the source spells
/// them) as a marker field.
///
/// The base rides in every variant rather than being factored out:
/// serde's flatten would consume the base's OPTIONAL `agent_id` and
/// `agent_type` before a variant that requires them could see them,
/// so the intersection is spelled per variant, requiredness and all.
///
/// The `tool_input`/`tool_response` fields are
/// `Option<serde_json::Value>`: the source types them bare
/// `z.unknown()`, which is a shape nobody promises AND a key nobody
/// requires — a zod object with an `unknown` field accepts its
/// absence, so this port does too.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum HookInput {
    /// Before a tool runs.
    PreToolUse {
        /// Always `PreToolUse`.
        hook_event_name: PreToolUseEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The tool about to run.
        tool_name: String,
        /// Its input, whatever the tool says it is. Optional the
        /// way every bare `z.unknown()` is: the key may be absent.
        tool_input: Option<serde_json::Value>,
        /// The call's id.
        tool_use_id: String,
    },
    /// After a tool runs.
    PostToolUse {
        /// Always `PostToolUse`.
        hook_event_name: PostToolUseEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The tool that ran.
        tool_name: String,
        /// Its input. Optional the way every bare `z.unknown()` is.
        tool_input: Option<serde_json::Value>,
        /// What it answered. Optional the way every bare
        /// `z.unknown()` is.
        tool_response: Option<serde_json::Value>,
        /// The call's id.
        tool_use_id: String,
    },
    /// After a tool fails.
    PostToolUseFailure {
        /// Always `PostToolUseFailure`.
        hook_event_name: PostToolUseFailureEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The tool that failed.
        tool_name: String,
        /// Its input. Optional the way every bare `z.unknown()` is.
        tool_input: Option<serde_json::Value>,
        /// The call's id.
        tool_use_id: String,
        /// What went wrong.
        error: String,
        /// Whether an interrupt is what failed it.
        is_interrupt: Option<bool>,
    },
    /// A permission was denied.
    PermissionDenied {
        /// Always `PermissionDenied`.
        hook_event_name: PermissionDeniedEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The tool that was refused.
        tool_name: String,
        /// Its input. Optional the way every bare `z.unknown()` is.
        tool_input: Option<serde_json::Value>,
        /// The call's id.
        tool_use_id: String,
        /// Why it was refused.
        reason: String,
    },
    /// A notification fired.
    Notification {
        /// Always `Notification`.
        hook_event_name: NotificationEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The notification's text.
        message: String,
        /// Its title.
        title: Option<String>,
        /// Its kind.
        notification_type: String,
    },
    /// A prompt was submitted.
    UserPromptSubmit {
        /// Always `UserPromptSubmit`.
        hook_event_name: UserPromptSubmitEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The prompt.
        prompt: String,
    },
    /// A session started.
    SessionStart {
        /// Always `SessionStart`.
        hook_event_name: SessionStartEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// How it started.
        source: SessionStartSource,
        /// The model it started with.
        model: Option<String>,
    },
    /// A session ended.
    SessionEnd {
        /// Always `SessionEnd`.
        hook_event_name: SessionEndEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// Why it ended.
        reason: ExitReason,
    },
    /// The turn stopped.
    Stop {
        /// Always `Stop`.
        hook_event_name: StopEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// Whether a stop hook is already running.
        stop_hook_active: bool,
        /// The last assistant message's text, so the hook need not
        /// parse the transcript.
        last_assistant_message: Option<String>,
    },
    /// The turn stopped on a failure.
    StopFailure {
        /// Always `StopFailure`.
        hook_event_name: StopFailureEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The failure's kind, in the assistant record's vocabulary.
        error: AssistantMessageError,
        /// The failure, in words.
        error_details: Option<String>,
        /// The last assistant message's text.
        last_assistant_message: Option<String>,
    },
    /// A subagent started. The base's optional agent fields are
    /// REQUIRED here — the intersection makes them so.
    SubagentStart {
        /// Always `SubagentStart`.
        hook_event_name: SubagentStartEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent.
        agent_id: String,
        /// Its type.
        agent_type: String,
    },
    /// A subagent stopped. The agent fields are required, as above.
    SubagentStop {
        /// Always `SubagentStop`.
        hook_event_name: SubagentStopEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// Whether a stop hook is already running.
        stop_hook_active: bool,
        /// The subagent.
        agent_id: String,
        /// Its transcript's path on disk.
        agent_transcript_path: String,
        /// Its type.
        agent_type: String,
        /// Its last assistant message's text.
        last_assistant_message: Option<String>,
    },
    /// Before compaction.
    PreCompact {
        /// Always `PreCompact`.
        hook_event_name: PreCompactEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// Who asked for it.
        trigger: CompactTrigger,
        /// The user's compaction instructions, or `null`.
        custom_instructions: Option<String>,
    },
    /// After compaction.
    PostCompact {
        /// Always `PostCompact`.
        hook_event_name: PostCompactEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// Who asked for it.
        trigger: CompactTrigger,
        /// The summary compaction produced.
        compact_summary: String,
    },
    /// A permission is being asked.
    PermissionRequest {
        /// Always `PermissionRequest`.
        hook_event_name: PermissionRequestEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The tool asking.
        tool_name: String,
        /// Its input. Optional the way every bare `z.unknown()` is.
        tool_input: Option<serde_json::Value>,
        /// Rules that would allow it, ready to apply.
        permission_suggestions: Option<Vec<PermissionUpdate>>,
    },
    /// Setup ran.
    Setup {
        /// Always `Setup`.
        hook_event_name: SetupEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// Why it ran.
        trigger: SetupTrigger,
    },
    /// A teammate went idle.
    TeammateIdle {
        /// Always `TeammateIdle`.
        hook_event_name: TeammateIdleEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The teammate.
        teammate_name: String,
        /// Their team.
        team_name: String,
    },
    /// A task was created.
    TaskCreated {
        /// Always `TaskCreated`.
        hook_event_name: TaskCreatedEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The task.
        task_id: String,
        /// Its subject.
        task_subject: String,
        /// Its description.
        task_description: Option<String>,
        /// The teammate involved, if any.
        teammate_name: Option<String>,
        /// Their team.
        team_name: Option<String>,
    },
    /// A task completed.
    TaskCompleted {
        /// Always `TaskCompleted`.
        hook_event_name: TaskCompletedEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The task.
        task_id: String,
        /// Its subject.
        task_subject: String,
        /// Its description.
        task_description: Option<String>,
        /// The teammate involved, if any.
        teammate_name: Option<String>,
        /// Their team.
        team_name: Option<String>,
    },
    /// An MCP server requested user input.
    Elicitation {
        /// Always `Elicitation`.
        hook_event_name: ElicitationEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The server eliciting.
        mcp_server_name: String,
        /// What it wants to say.
        message: String,
        /// Form or URL mode.
        mode: Option<super::ElicitationMode>,
        /// The URL, in URL mode.
        url: Option<String>,
        /// The elicitation's id.
        elicitation_id: Option<String>,
        /// The schema the answer should satisfy.
        requested_schema:
            Option<indexmap::IndexMap<String, serde_json::Value>>,
    },
    /// The user answered an MCP elicitation.
    ElicitationResult {
        /// Always `ElicitationResult`.
        hook_event_name: ElicitationResultEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The server that elicited.
        mcp_server_name: String,
        /// The elicitation's id.
        elicitation_id: Option<String>,
        /// Form or URL mode.
        mode: Option<super::ElicitationMode>,
        /// What the user did.
        action: ElicitationAction,
        /// What they entered.
        content: Option<indexmap::IndexMap<String, serde_json::Value>>,
    },
    /// Configuration changed on disk.
    ConfigChange {
        /// Always `ConfigChange`.
        hook_event_name: ConfigChangeEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// Which settings family changed.
        source: ConfigChangeSource,
        /// The file that changed, when one did.
        file_path: Option<String>,
    },
    /// Memory instructions were loaded.
    InstructionsLoaded {
        /// Always `InstructionsLoaded`.
        hook_event_name: InstructionsLoadedEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The file loaded.
        file_path: String,
        /// Which memory tier it belongs to.
        memory_type: InstructionsMemoryType,
        /// Why it loaded.
        load_reason: InstructionsLoadReason,
        /// The globs that made it conditional, when any.
        globs: Option<Vec<String>>,
        /// The file whose access triggered the load.
        trigger_file_path: Option<String>,
        /// The file whose include pulled it in.
        parent_file_path: Option<String>,
    },
    /// A worktree was created.
    WorktreeCreate {
        /// Always `WorktreeCreate`.
        hook_event_name: WorktreeCreateEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The worktree's name.
        name: String,
    },
    /// A worktree was removed.
    WorktreeRemove {
        /// Always `WorktreeRemove`.
        hook_event_name: WorktreeRemoveEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The worktree's path.
        worktree_path: String,
    },
    /// The working directory changed.
    CwdChanged {
        /// Always `CwdChanged`.
        hook_event_name: CwdChangedEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// Where it was.
        old_cwd: String,
        /// Where it is.
        new_cwd: String,
    },
    /// A watched file changed.
    FileChanged {
        /// Always `FileChanged`.
        hook_event_name: FileChangedEventName,
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        agent_type: Option<String>,
        /// The file.
        file_path: String,
        /// What happened to it.
        event: FileChangeEvent,
    },
}

/// How a session started.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SessionStartSource {
    /// Fresh.
    Startup,
    /// Resumed.
    Resume,
    /// After a clear.
    Clear,
    /// After a compaction.
    Compact,
}

/// Why a session ended — the source's exit reasons.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExitReason {
    /// A clear.
    Clear,
    /// A resume.
    Resume,
    /// A logout.
    Logout,
    /// The prompt input exited.
    PromptInputExit,
    /// Something else.
    Other,
    /// Bypass-permissions was disabled.
    BypassPermissionsDisabled,
}

/// Who asked for a compaction — the hooks' copy of the trigger.
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

/// Why Setup ran.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SetupTrigger {
    /// First-time initialization.
    Init,
    /// Maintenance.
    Maintenance,
}

/// What the user did with an elicitation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ElicitationAction {
    /// Answered it.
    Accept,
    /// Declined it.
    Decline,
    /// Cancelled it.
    Cancel,
}

/// Which settings family a ConfigChange saw move.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ConfigChangeSource {
    /// The user's settings.
    UserSettings,
    /// The project's shared settings.
    ProjectSettings,
    /// The project's gitignored settings.
    LocalSettings,
    /// Managed policy settings.
    PolicySettings,
    /// Skills.
    Skills,
}

/// Which memory tier an InstructionsLoaded file belongs to.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
pub enum InstructionsMemoryType {
    /// The user tier.
    User,
    /// The project tier.
    Project,
    /// The local tier.
    Local,
    /// The managed tier.
    Managed,
}

/// Why an InstructionsLoaded file loaded.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum InstructionsLoadReason {
    /// At session start.
    SessionStart,
    /// Walking into a nested directory.
    NestedTraversal,
    /// A path glob matched.
    PathGlobMatch,
    /// An include pulled it in.
    Include,
    /// After a compaction.
    Compact,
}

/// What happened to a watched file.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeEvent {
    /// It changed.
    Change,
    /// It appeared.
    Add,
    /// It vanished.
    Unlink,
}

/// The `PreToolUse` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PreToolUseEventName {
    /// The only value.
    #[default]
    PreToolUse,
}

/// The `PostToolUse` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PostToolUseEventName {
    /// The only value.
    #[default]
    PostToolUse,
}

/// The `PostToolUseFailure` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PostToolUseFailureEventName {
    /// The only value.
    #[default]
    PostToolUseFailure,
}

/// The `PermissionDenied` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PermissionDeniedEventName {
    /// The only value.
    #[default]
    PermissionDenied,
}

/// The `Notification` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum NotificationEventName {
    /// The only value.
    #[default]
    Notification,
}

/// The `UserPromptSubmit` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum UserPromptSubmitEventName {
    /// The only value.
    #[default]
    UserPromptSubmit,
}

/// The `SessionStart` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum SessionStartEventName {
    /// The only value.
    #[default]
    SessionStart,
}

/// The `SessionEnd` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum SessionEndEventName {
    /// The only value.
    #[default]
    SessionEnd,
}

/// The `Stop` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum StopEventName {
    /// The only value.
    #[default]
    Stop,
}

/// The `StopFailure` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum StopFailureEventName {
    /// The only value.
    #[default]
    StopFailure,
}

/// The `SubagentStart` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum SubagentStartEventName {
    /// The only value.
    #[default]
    SubagentStart,
}

/// The `SubagentStop` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum SubagentStopEventName {
    /// The only value.
    #[default]
    SubagentStop,
}

/// The `PreCompact` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PreCompactEventName {
    /// The only value.
    #[default]
    PreCompact,
}

/// The `PostCompact` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PostCompactEventName {
    /// The only value.
    #[default]
    PostCompact,
}

/// The `PermissionRequest` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum PermissionRequestEventName {
    /// The only value.
    #[default]
    PermissionRequest,
}

/// The `Setup` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum SetupEventName {
    /// The only value.
    #[default]
    Setup,
}

/// The `TeammateIdle` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum TeammateIdleEventName {
    /// The only value.
    #[default]
    TeammateIdle,
}

/// The `TaskCreated` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum TaskCreatedEventName {
    /// The only value.
    #[default]
    TaskCreated,
}

/// The `TaskCompleted` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum TaskCompletedEventName {
    /// The only value.
    #[default]
    TaskCompleted,
}

/// The `Elicitation` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum ElicitationEventName {
    /// The only value.
    #[default]
    Elicitation,
}

/// The `ElicitationResult` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum ElicitationResultEventName {
    /// The only value.
    #[default]
    ElicitationResult,
}

/// The `ConfigChange` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum ConfigChangeEventName {
    /// The only value.
    #[default]
    ConfigChange,
}

/// The `InstructionsLoaded` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum InstructionsLoadedEventName {
    /// The only value.
    #[default]
    InstructionsLoaded,
}

/// The `WorktreeCreate` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum WorktreeCreateEventName {
    /// The only value.
    #[default]
    WorktreeCreate,
}

/// The `WorktreeRemove` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum WorktreeRemoveEventName {
    /// The only value.
    #[default]
    WorktreeRemove,
}

/// The `CwdChanged` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum CwdChangedEventName {
    /// The only value.
    #[default]
    CwdChanged,
}

/// The `FileChanged` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum FileChangedEventName {
    /// The only value.
    #[default]
    FileChanged,
}
