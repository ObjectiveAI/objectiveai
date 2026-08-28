//! What a hook callback is handed: one input per lifecycle event.

use serde::{Deserialize, Serialize};

use super::super::assistant::AssistantMessageError;
use super::config::PermissionUpdate;

/// A hook's input, discriminated by `hook_event_name` — the source's
/// twenty-seven, each the base fields intersected with the event's
/// own.
///
/// The base rides in every variant rather than being factored out:
/// serde's flatten would consume the base's OPTIONAL `agent_id` and
/// `agent_type` before a variant that requires them could see them,
/// so the intersection is spelled per variant, requiredness and all.
///
/// The `tool_input`/`tool_response` fields stay
/// [`serde_json::Value`]: the source types them `unknown`, because
/// their shape belongs to whichever tool is involved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum HookInput {
    /// Before a tool runs.
    PreToolUse {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The tool about to run.
        tool_name: String,
        /// Its input, whatever the tool says it is.
        tool_input: serde_json::Value,
        /// The call's id.
        tool_use_id: String,
    },
    /// After a tool runs.
    PostToolUse {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The tool that ran.
        tool_name: String,
        /// Its input.
        tool_input: serde_json::Value,
        /// What it answered.
        tool_response: serde_json::Value,
        /// The call's id.
        tool_use_id: String,
    },
    /// After a tool fails.
    PostToolUseFailure {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The tool that failed.
        tool_name: String,
        /// Its input.
        tool_input: serde_json::Value,
        /// The call's id.
        tool_use_id: String,
        /// What went wrong.
        error: String,
        /// Whether an interrupt is what failed it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is_interrupt: Option<bool>,
    },
    /// A permission was denied.
    PermissionDenied {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The tool that was refused.
        tool_name: String,
        /// Its input.
        tool_input: serde_json::Value,
        /// The call's id.
        tool_use_id: String,
        /// Why it was refused.
        reason: String,
    },
    /// A notification fired.
    Notification {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The notification's text.
        message: String,
        /// Its title.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Its kind.
        notification_type: String,
    },
    /// A prompt was submitted.
    UserPromptSubmit {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The prompt.
        prompt: String,
    },
    /// A session started.
    SessionStart {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// How it started.
        source: SessionStartSource,
        /// The model it started with.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    /// A session ended.
    SessionEnd {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// Why it ended.
        reason: ExitReason,
    },
    /// The turn stopped.
    Stop {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// Whether a stop hook is already running.
        stop_hook_active: bool,
        /// The last assistant message's text, so the hook need not
        /// parse the transcript.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_assistant_message: Option<String>,
    },
    /// The turn stopped on a failure.
    StopFailure {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The failure's kind, in the assistant record's vocabulary.
        error: AssistantMessageError,
        /// The failure, in words.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error_details: Option<String>,
        /// The last assistant message's text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_assistant_message: Option<String>,
    },
    /// A subagent started. The base's optional agent fields are
    /// REQUIRED here — the intersection makes them so.
    SubagentStart {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent.
        agent_id: String,
        /// Its type.
        agent_type: String,
    },
    /// A subagent stopped. The agent fields are required, as above.
    SubagentStop {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_assistant_message: Option<String>,
    },
    /// Before compaction.
    PreCompact {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// Who asked for it.
        trigger: CompactTrigger,
        /// The user's compaction instructions, or `null`.
        custom_instructions: Option<String>,
    },
    /// After compaction.
    PostCompact {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// Who asked for it.
        trigger: CompactTrigger,
        /// The summary compaction produced.
        compact_summary: String,
    },
    /// A permission is being asked.
    PermissionRequest {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The tool asking.
        tool_name: String,
        /// Its input.
        tool_input: serde_json::Value,
        /// Rules that would allow it, ready to apply.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_suggestions: Option<Vec<PermissionUpdate>>,
    },
    /// Setup ran.
    Setup {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// Why it ran.
        trigger: SetupTrigger,
    },
    /// A teammate went idle.
    TeammateIdle {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The teammate.
        teammate_name: String,
        /// Their team.
        team_name: String,
    },
    /// A task was created.
    TaskCreated {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The task.
        task_id: String,
        /// Its subject.
        task_subject: String,
        /// Its description.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_description: Option<String>,
        /// The teammate involved, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        teammate_name: Option<String>,
        /// Their team.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        team_name: Option<String>,
    },
    /// A task completed.
    TaskCompleted {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The task.
        task_id: String,
        /// Its subject.
        task_subject: String,
        /// Its description.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_description: Option<String>,
        /// The teammate involved, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        teammate_name: Option<String>,
        /// Their team.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        team_name: Option<String>,
    },
    /// An MCP server requested user input.
    Elicitation {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The server eliciting.
        mcp_server_name: String,
        /// What it wants to say.
        message: String,
        /// Form or URL mode.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<super::ElicitationMode>,
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
    /// The user answered an MCP elicitation.
    ElicitationResult {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The server that elicited.
        mcp_server_name: String,
        /// The elicitation's id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        elicitation_id: Option<String>,
        /// Form or URL mode.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<super::ElicitationMode>,
        /// What the user did.
        action: ElicitationAction,
        /// What they entered.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<indexmap::IndexMap<String, serde_json::Value>>,
    },
    /// Configuration changed on disk.
    ConfigChange {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// Which settings family changed.
        source: ConfigChangeSource,
        /// The file that changed, when one did.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file_path: Option<String>,
    },
    /// Memory instructions were loaded.
    InstructionsLoaded {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The file loaded.
        file_path: String,
        /// Which memory tier it belongs to.
        memory_type: InstructionsMemoryType,
        /// Why it loaded.
        load_reason: InstructionsLoadReason,
        /// The globs that made it conditional, when any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        globs: Option<Vec<String>>,
        /// The file whose access triggered the load.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trigger_file_path: Option<String>,
        /// The file whose include pulled it in.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_file_path: Option<String>,
    },
    /// A worktree was created.
    WorktreeCreate {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The worktree's name.
        name: String,
    },
    /// A worktree was removed.
    WorktreeRemove {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The worktree's path.
        worktree_path: String,
    },
    /// The working directory changed.
    CwdChanged {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// Where it was.
        old_cwd: String,
        /// Where it is.
        new_cwd: String,
    },
    /// A watched file changed.
    FileChanged {
        /// The session.
        session_id: String,
        /// The transcript's path on disk.
        transcript_path: String,
        /// The working directory.
        cwd: String,
        /// The permission mode in force.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_mode: Option<String>,
        /// The subagent, when the hook fired inside one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The agent type, inside a subagent or an `--agent` session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
        /// The file.
        file_path: String,
        /// What happened to it.
        event: FileChangeEvent,
    },
}

/// How a session started.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
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
