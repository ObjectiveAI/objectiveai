//! Claude Code's built-in tools, each a switch.

use serde::{Deserialize, Serialize};

/// The built-in tools Claude Code is given, every one stated.
///
/// Claude Code's `--tools` flag takes the exact set of built-ins the
/// model may call, and this is that set: a switch per tool, each an
/// `Option<bool>` where ABSENT IS OFF — the same as `false` — so
/// nothing is on by omission and a request names only what it
/// turns on. The harness renders the `true` switches as the flag's
/// list, in this order, after the always-on names.
///
/// Always on, no switch: the `mcp__*` tools — the caller's own
/// tools, served through the proxy — are not built-ins and are
/// present whatever `--tools` says: the tool channel is what the
/// protocol is built around. Beside them the harness always lists
/// the four MCP resource built-ins (`ListMcpResources`,
/// `ReadMcpResource`, `ReadMcpResourceDir`, `RefreshMcpTools` — the
/// caller's resources, through the same proxy), `Skill` (skills are
/// the directories mounted under `~/.claude/skills`; the tool is
/// offered whether or not any are, and a call names a skill that is
/// not there fails as "unknown skill"), and `ToolSearch`, by which
/// the model loads deferred tools' schemas.
///
/// Always off, no switch: `LSP` (nothing in the image installs a
/// language server). Nor does a switch exist for what cannot work
/// in the container, which the harness never lists:
/// `AskUserQuestion`, `EnterPlanMode`,
/// `ExitPlanMode`, `ProposeGoal` and `ProposeSkills` (a call waits
/// on a user who is not there); `CronCreate`, `CronDelete`,
/// `CronList` and `ScheduleWakeup` (the container does not outlive
/// the run); `Artifact`, `Projects`, `ClaudeDesign`, `RemoteTrigger`,
/// `PushNotification`, `SendFeedback`, `ReadNotifications`,
/// `ShowOnboardingRolePicker` and `EndConversation` (claude.ai
/// account and host features). Nor for the legacy names the binary
/// still answers to (`MultiEdit`, `BashOutput`, `KillShell`): they
/// are the old spellings of switches here.
///
/// Whether the pinned build honors every name here under `-p` with
/// stream-json input is a live-run check, not a promise of this
/// type; the names are as the runtime spells them.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
pub struct Tools {
    /// `Bash`: runs a command, with an optional timeout and
    /// optionally in the background.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bash: Option<bool>,
    /// `Read`: reads a file by absolute path, with `offset` and `limit`
    /// for large files and `pages` for PDFs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<bool>,
    /// `Write`: writes a whole file, overwriting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write: Option<bool>,
    /// `Edit`: exact-string replacement in a file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<bool>,
    /// `NotebookEdit`: replaces, inserts or deletes one Jupyter cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notebook_edit: Option<bool>,
    /// `Glob`: file pattern matching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<bool>,
    /// `Grep`: ripgrep over the workspace (ripgrep ships with the
    /// binary).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grep: Option<bool>,
    /// `Task` (also spelled `Agent`): launches a sub-agent. Its chunks
    /// reach the run's stream attributed by `parent_tool_call_id`.
    /// Local sub-agents only: `isolation: remote` needs a cloud
    /// environment the container has no account for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<bool>,
    /// `SendMessage`: messages a running named agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_message: Option<bool>,
    /// `ListAgents`: lists addressable agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_agents: Option<bool>,
    /// `TaskOutput`: reads the output of a background task, blocking
    /// or not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_output: Option<bool>,
    /// `TaskStop`: stops a background task by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_stop: Option<bool>,
    /// `Monitor`: watches a command's stdout lines or a WebSocket's
    /// frames as events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<bool>,
    /// `Workflow`: runs a deterministic multi-agent orchestration
    /// script.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<bool>,
    /// `TodoWrite`: the session's structured todo list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todo_write: Option<bool>,
    /// `TaskCreate`: creates an entry on the session's task board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_create: Option<bool>,
    /// `TaskGet`: reads one task board entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_get: Option<bool>,
    /// `TaskUpdate`: updates one task board entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_update: Option<bool>,
    /// `TaskList`: lists the task board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_list: Option<bool>,
    /// `ReportFindings`: reports code-review findings as a typed list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_findings: Option<bool>,
    /// `REPL`: runs JavaScript with persistent state and top-level
    /// await.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repl: Option<bool>,
    /// `WebFetch`: fetches a URL and runs a prompt over the content.
    /// Needs outbound network from the container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_fetch: Option<bool>,
    /// `WebSearch`: searches the web, with domain allow and block
    /// lists. Needs outbound network from the container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_search: Option<bool>,
    /// `EnterWorktree`: creates or switches into a git worktree. Needs
    /// the workspace to be a git repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter_worktree: Option<bool>,
    /// `ExitWorktree`: leaves the worktree, keeping or removing it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_worktree: Option<bool>,
    /// `Sleep`: waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sleep: Option<bool>,
}

impl Tools {
    /// The runtime names of the tools switched on, in declaration
    /// order: the value of `--tools`, less the always-on names the
    /// harness adds.
    pub fn names(&self) -> Vec<&'static str> {
        [
            (self.bash, "Bash"),
            (self.read, "Read"),
            (self.write, "Write"),
            (self.edit, "Edit"),
            (self.notebook_edit, "NotebookEdit"),
            (self.glob, "Glob"),
            (self.grep, "Grep"),
            (self.task, "Task"),
            (self.send_message, "SendMessage"),
            (self.list_agents, "ListAgents"),
            (self.task_output, "TaskOutput"),
            (self.task_stop, "TaskStop"),
            (self.monitor, "Monitor"),
            (self.workflow, "Workflow"),
            (self.todo_write, "TodoWrite"),
            (self.task_create, "TaskCreate"),
            (self.task_get, "TaskGet"),
            (self.task_update, "TaskUpdate"),
            (self.task_list, "TaskList"),
            (self.report_findings, "ReportFindings"),
            (self.repl, "REPL"),
            (self.web_fetch, "WebFetch"),
            (self.web_search, "WebSearch"),
            (self.enter_worktree, "EnterWorktree"),
            (self.exit_worktree, "ExitWorktree"),
            (self.sleep, "Sleep"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.unwrap_or(false).then_some(name))
        .collect()
    }
}
