//! Claude Code's built-in tools, each a switch.

use serde::{Deserialize, Serialize};

/// The built-in tools Claude Code is given, every one stated.
///
/// Claude Code's `--tools` flag takes the exact set of built-ins the
/// model may call, and this is that set: a switch per tool, none
/// optional, so a request says what the model sees and nothing is
/// on by omission. The harness renders the `true` switches as the
/// flag's list, in this order. The `mcp__*` tools — the caller's own
/// tools, served through the proxy — are not built-ins and are
/// always present: the tool channel is what the protocol is built
/// around. `ToolSearch`, by which the model loads deferred tools'
/// schemas, is likewise always listed by the harness.
///
/// No switch exists for what cannot work in the container, and the
/// harness never lists it: `AskUserQuestion`, `EnterPlanMode`,
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
    pub bash: bool,
    /// `Read`: reads a file by absolute path, with `offset` and `limit`
    /// for large files and `pages` for PDFs.
    pub read: bool,
    /// `Write`: writes a whole file, overwriting.
    pub write: bool,
    /// `Edit`: exact-string replacement in a file.
    pub edit: bool,
    /// `NotebookEdit`: replaces, inserts or deletes one Jupyter cell.
    pub notebook_edit: bool,
    /// `Glob`: file pattern matching.
    pub glob: bool,
    /// `Grep`: ripgrep over the workspace (ripgrep ships with the
    /// binary).
    pub grep: bool,
    /// `Task` (also spelled `Agent`): launches a sub-agent. Its chunks
    /// reach the run's stream attributed by `parent_tool_call_id`.
    /// Local sub-agents only: `isolation: remote` needs a cloud
    /// environment the container has no account for.
    pub task: bool,
    /// `SendMessage`: messages a running named agent.
    pub send_message: bool,
    /// `ListAgents`: lists addressable agents.
    pub list_agents: bool,
    /// `TaskOutput`: reads the output of a background task, blocking
    /// or not.
    pub task_output: bool,
    /// `TaskStop`: stops a background task by id.
    pub task_stop: bool,
    /// `Monitor`: watches a command's stdout lines or a WebSocket's
    /// frames as events.
    pub monitor: bool,
    /// `Workflow`: runs a deterministic multi-agent orchestration
    /// script.
    pub workflow: bool,
    /// `TodoWrite`: the session's structured todo list.
    pub todo_write: bool,
    /// `TaskCreate`: creates an entry on the session's task board.
    pub task_create: bool,
    /// `TaskGet`: reads one task board entry.
    pub task_get: bool,
    /// `TaskUpdate`: updates one task board entry.
    pub task_update: bool,
    /// `TaskList`: lists the task board.
    pub task_list: bool,
    /// `ReportFindings`: reports code-review findings as a typed list.
    pub report_findings: bool,
    /// `REPL`: runs JavaScript with persistent state and top-level
    /// await.
    pub repl: bool,
    /// `WebFetch`: fetches a URL and runs a prompt over the content.
    /// Needs outbound network from the container.
    pub web_fetch: bool,
    /// `WebSearch`: searches the web, with domain allow and block
    /// lists. Needs outbound network from the container.
    pub web_search: bool,
    /// `ListMcpResources`: lists resources across the MCP servers —
    /// here, the caller's, through the proxy.
    pub list_mcp_resources: bool,
    /// `ReadMcpResource`: reads one MCP resource by server and URI.
    pub read_mcp_resource: bool,
    /// `ReadMcpResourceDir`: lists a directory resource.
    pub read_mcp_resource_dir: bool,
    /// `RefreshMcpTools`: re-lists tools from one or all MCP servers.
    pub refresh_mcp_tools: bool,
    /// `EnterWorktree`: creates or switches into a git worktree. Needs
    /// the workspace to be a git repository.
    pub enter_worktree: bool,
    /// `ExitWorktree`: leaves the worktree, keeping or removing it.
    pub exit_worktree: bool,
    /// `LSP`: language-server queries. Works only where a language
    /// server is installed; the image installs none.
    pub lsp: bool,
    /// `Skill`: invokes a skill by name. Skills are the directories
    /// mounted under `~/.claude/skills`.
    pub skill: bool,
    /// `Sleep`: waits.
    pub sleep: bool,
}

impl Tools {
    /// The runtime names of the tools switched on, in declaration
    /// order: the value of `--tools`, less the always-on
    /// `ToolSearch` the harness appends.
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
            (self.list_mcp_resources, "ListMcpResources"),
            (self.read_mcp_resource, "ReadMcpResource"),
            (self.read_mcp_resource_dir, "ReadMcpResourceDir"),
            (self.refresh_mcp_tools, "RefreshMcpTools"),
            (self.enter_worktree, "EnterWorktree"),
            (self.exit_worktree, "ExitWorktree"),
            (self.lsp, "LSP"),
            (self.skill, "Skill"),
            (self.sleep, "Sleep"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect()
    }
}
