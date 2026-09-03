# Claude Code's tools — every one, as the pinned build ships them

From `@anthropic-ai/claude-code@2.1.246`, the package
`spawn/install.rs` installs at container start. Two sources, both
fetched on 2026-09-03: the npm wrapper's `sdk-tools.d.ts` (the
generated input and output schema of every tool, with the field
docs the model is shown) and the `linux-x64` native binary the
wrapper downloads (the tool NAMES as the runtime spells them, their
descriptions, and the CLI's selection flags). Nothing here is from
memory of the product.

## What our container does today

Everything on. The argv in `spawn/spawn.rs` passes
`--dangerously-skip-permissions` (no prompts), `--mcp-config`
pointing at the proxy, `--model`, `--thinking`, `--effort`,
`--resume` — and no tool flag at all, so every built-in the build
enables by default is live inside the container, plus whatever the
MCP proxy serves. The `claude_code` agent in the SDK carries `model`,
`thinking`, `effort`; it has no toolset vocabulary, so a caller
cannot turn anything off and the harness does not either.

## The selection flags

The binary carries three, each present in its help table:

- `--allowedTools` / `--allowed-tools <tools...>` — the allowlist:
  tool names, with rule forms like `Bash(git:*)`, that run without
  asking. Under `--dangerously-skip-permissions` nothing asks
  anyway, so this one is moot here.
- `--disallowedTools` / `--disallowed-tools <tools...>` — the
  denylist: named tools are removed from what the model can call.
  This is the switch a harness uses to turn a built-in OFF.
- `--tools <tools...>` — "Specify the list of available tools from
  the built-in set": the whole set, replaced by exactly the names
  given. The positive form of the same switch.

Names are matched as the runtime spells them — the first column
below.

## The tools, by what they do

Each row: the runtime name (confirmed as a quoted string in the
binary), the schema type in `sdk-tools.d.ts`, what it does, and
whether it can work inside this container as built. "Needs" means a
precondition the container does not currently satisfy.

### Files

| name | schema | does | in the container |
|---|---|---|---|
| `Read` | `FileReadInput` | reads a file by absolute path, with `offset`/`limit` for large files and `pages` for PDFs | works |
| `Write` | `FileWriteInput` | writes a whole file, overwriting | works |
| `Edit` | `FileEditInput` | exact-string replacement in a file (`replace_all` optional) | works |
| `NotebookEdit` | `NotebookEditInput` | replace/insert/delete one Jupyter cell by id | works |
| `Glob` | `GlobInput` | file pattern matching, any codebase size | works |
| `Grep` | `GrepInput` | ripgrep with content/files/count modes, context, type filters, multiline | works (ripgrep ships with the binary) |

`MultiEdit` survives in the binary as a name (6 occurrences) but has
no schema in this build — a legacy alias, not a tool the model is
offered.

### The shell, and background tasks

| name | schema | does | in the container |
|---|---|---|---|
| `Bash` | `BashInput` | runs a command with optional timeout (max 600s), optionally in the background, optionally with `dangerouslyDisableSandbox` | works; the description in this build is "Executes a given bash command and returns its output" (a PowerShell variant exists for Windows hosts) |
| `TaskOutput` | `TaskOutputInput` | reads the output of a background task (shell, agent, or remote session), blocking or not | works |
| `TaskStop` | `TaskStopInput` | stops a background task by id | works |

`BashOutput` and `KillShell` remain as names (2 each) — the previous
names of these two, kept for compatibility.

### Agents and coordination

| name | schema | does | in the container |
|---|---|---|---|
| `Task` (also spelled `Agent`) | `AgentInput` | launches a sub-agent with a prompt, a `subagent_type`, an optional model override, background by default, optional `isolation` (`worktree` or `remote`) | works for local sub-agents; `isolation: remote` needs a cloud environment the container has no account for. The sub-agent's chunks reach our stream attributed by `parent_tool_call_id` |
| `SendMessage` | — (untyped) | messages a running named agent | works with the above |
| `ListAgents` | — (untyped) | lists addressable agents | works |
| `Workflow` | `WorkflowInput` | runs a deterministic multi-agent orchestration script (`agent()`/`parallel()`/`pipeline()`) | works locally; the user must have opted in by the tool's own rules |
| `Monitor` | `MonitorInput` | watches a command's stdout lines or a WebSocket's frames as events, with a timeout or persistent | works |

### Planning, tasks, and the session

| name | schema | does | in the container |
|---|---|---|---|
| `EnterPlanMode` / `ExitPlanMode` | `EnterPlanModeInput` / `ExitPlanModeInput` | plan mode in and out; exit asks the user to approve a plan file | ExitPlanMode needs a user to answer — under `-p` with our stream there is nobody; a call would stall the turn |
| `AskUserQuestion` | `AskUserQuestionInput` | asks the user one to four questions with options | same: needs a user |
| `TodoWrite` | `TodoWriteInput` | the session's structured todo list | works |
| `TaskCreate` / `TaskGet` / `TaskUpdate` / `TaskList` | the four `Task*Input`s | a durable task board with blocking relations, owners, metadata | works (session-local state) |
| `ProposeGoal` | `ProposeGoalInput` | proposes a completion condition, by default asking the user to approve it | needs a user unless `ask_user: false` |
| `ProposeSkills` | `ProposeSkillsInput` | proposes new or improved skills as SKILL.md drafts | has a schema but NO name in this binary (0 occurrences): not offered in this build |
| `ReportFindings` | `ReportFindingsInput` | reports code-review findings as a typed list for the host UI | works, but only meaningful when a review skill drives it |
| `REPL` | `REPLInput` | runs JavaScript with persistent state and top-level await | works (the binary embeds its runtime) |

### The web

| name | schema | does | in the container |
|---|---|---|---|
| `WebFetch` | `WebFetchInput` | fetches a URL and runs a prompt over the content with a model | needs outbound network from the container, and the model call rides Claude Code's own credentials |
| `WebSearch` | `WebSearchInput` | searches the web with domain allow/block lists | same |

### MCP

| name | schema | does | in the container |
|---|---|---|---|
| `mcp__<server>__<tool>` | `McpInput` (open object) | every tool the configured servers expose — for us, the proxy's, which are the caller's | works: this is the tool channel the protocol is built around |
| `ListMcpResources` | `ListMcpResourcesInput` | lists resources across servers, optionally one | works through the proxy's `list_resources` relay |
| `ReadMcpResource` | `ReadMcpResourceInput` | reads one resource by server and URI | works through the proxy's `read_resource` relay |
| `ReadMcpResourceDir` | `ReadMcpResourceDirInput` | lists a directory resource | works if the caller's server serves one |
| `RefreshMcpTools` | `RefreshMcpToolsInput` | re-lists tools from one or all servers | works |
| `ToolSearch` | — (untyped) | fetches deferred tools' schemas on demand | works; internal to the model's tool loading |

### Automation and time

| name | schema | does | in the container |
|---|---|---|---|
| `CronCreate` / `CronDelete` / `CronList` | the three `Cron*Input`s | schedules prompts on a cron expression, in-memory or durable in `.claude/scheduled_tasks.json` | runs, but a fire after the run ends reaches nothing: the container's life is one run |
| `ScheduleWakeup` | `ScheduleWakeupInput` | the `/loop` self-pacing wake-up, 60–3600s | same objection |
| `Sleep` | — (untyped) | waits | works |

### Account, cloud, and host features

| name | schema | does | in the container |
|---|---|---|---|
| `Artifact` | `ArtifactInput` | publishes an HTML page to claude.ai, reads, lists, watches, manages assets and a database | needs a claude.ai account and network; foreign to a headless container |
| `Projects` | `ProjectsInput` | reads, writes, searches a claude.ai project's documents | same |
| `ClaudeDesign` | `ClaudeDesignInput` | Claude Design operations (server-validated) | same |
| `RemoteTrigger` | `RemoteTriggerInput` | manages remote triggers and their runs | same |
| `PushNotification` | `PushNotificationInput` | pushes a mobile notification | same |
| `SendFeedback` | `SendFeedbackInput` | drafts product feedback for the user to approve | needs a user |
| `ReadNotifications` | `ReadNotificationsInput` | reads the host's notifications | host feature |
| `ShowOnboardingRolePicker` | `ShowOnboardingRolePickerInput` | an onboarding UI | host feature |
| `EndConversation` | — (untyped) | ends the conversation | meaningless here |

### Worktrees and language servers

| name | schema | does | in the container |
|---|---|---|---|
| `EnterWorktree` / `ExitWorktree` | `EnterWorktreeInput` / `ExitWorktreeInput` | creates or switches into a git worktree; leaves it, keeping or removing | works when the workspace is a git repo |
| `LSP` | — (untyped) | language-server queries | works where a server is installed; nothing in the image installs one |
| `Skill` | — (untyped) | invokes a skill by name | works with skills mounted under `~/.claude/skills` |

## Counts

46 typed inputs in `sdk-tools.d.ts`; 44 of them carry a confirmed
runtime name (`ProposeSkills` has none; `McpInput` is the shape of
every MCP tool rather than one name). Nine further names exist in
the binary with no typed schema: `ToolSearch`, `LSP`, `SendMessage`,
`ListAgents`, `Skill`, `Sleep`, `EndConversation`, and the legacy
`MultiEdit`, `BashOutput`, `KillShell`.

## What this says about a toolset vocabulary

Three groups fall out of the table, if the `claude_code` agent were
to grow toolset switches the way `hermes` has them:

1. **Tools a caller might reasonably want off**: `Bash`, the file
   tools, `NotebookEdit`, `WebFetch`, `WebSearch`, `Task` (sub-agents),
   `REPL`, `Workflow`, the worktree pair — each a real capability
   with a real reason to withhold it. These are the candidates for
   `Option<bool>` switches, rendered as `--disallowedTools` when
   `false`.
2. **Tools that cannot work under `-p` in this container and should
   be OFF by the harness regardless**: `AskUserQuestion`,
   `ExitPlanMode` (a call stalls the turn waiting for a user who is
   not there), the cron and wake-up tools (the container does not
   outlive the run), and the account and host features (`Artifact`,
   `Projects`, `ClaudeDesign`, `RemoteTrigger`, `PushNotification`,
   `SendFeedback`, `ReadNotifications`, `ShowOnboardingRolePicker`).
3. **Tools that are the protocol's own or harmless**: the MCP family
   (the tool channel), `TodoWrite`, the task board, `Monitor`,
   `TaskOutput`/`TaskStop`, `Skill`, `LSP`, `Sleep` — left at their
   defaults.

Whether group 2 is actually offered to the model under `-p` with
stream-json input is the one thing this report could not settle from
the binary: the tool registry's per-mode gating is code, not
strings. It is the first thing a live run should check, by listing
what the model was given.
