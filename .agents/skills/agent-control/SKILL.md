---
name: agent-control
description: Drive and inspect ObjectiveAI agents from the CLI — spawn, message, wait, tags, and above all the persisted `agents logs` tier for reading exactly what an agent did. Use when running an agent, or when an agent has run and you need its tool calls and results.
---

# Controlling agents

## Run the command directly. Do NOT redirect to a file.

`agents spawn` and `agents message` print **one short line** — an agent
instance hierarchy in quotes, nothing else. Backgrounding them to a log
file and then reading the file back costs a spawn, a poll, a read, and
usually a couple of retries: **strictly more tokens than just running
the command and reading its output.**

```bash
# RIGHT — one call, one line back, the id is right there
objectiveai agents spawn --agent-file ./my-agent.json --simple "go"
# -> "daemon/3Bk4hBIOrMDdNaDL3atd1b-F1LTN4MMGSB1wpHXF"

# WRONG — the id you need is now in a file you have to go fetch
nohup objectiveai agents spawn --agent-file ./my-agent.json --simple "go" > out.log 2>&1 &
```

Background + log file is for genuinely long, chatty commands (builds,
installs). It is the wrong shape for these. The same goes for wrapping
them in polling loops: **keep the id the command hands you** — every
later lookup keys off it.

Blocking is fine. A spawn returns when the completion is done; that is
usually seconds.

## Reading what an agent did — `agents logs`

This is the payoff, and the target syntax is the one thing that
reliably wastes time.

**A spawn prints `daemon/<agent>-<response>`. That whole string is the
agent instance hierarchy, and it is NOT what `--target instance=` takes.
Split it: the last segment is the INSTANCE, everything before it is the
PARENT.**

```bash
# spawn printed: "daemon/3Bk4hBIOrMDdNaDL3atd1b-F1LTN4MMGSB1wpHXF"
#                 ^parent  ^instance

# RIGHT
objectiveai agents logs list \
  --target "instance=3Bk4hBIOrMDdNaDL3atd1b-F1LTN4MMGSB1wpHXF,parent=daemon" --all

# WRONG — silently returns NOTHING, exit 0, no error
objectiveai agents logs list \
  --target "instance=daemon/3Bk4hBIOrMDdNaDL3atd1b-F1LTN4MMGSB1wpHXF" --all
```

The wrong form does not fail. It prints nothing and exits 0, which is
indistinguishable from "the agent logged nothing" — so an empty result
is far more likely to be a bad target than a quiet agent. `parent`
defaults to the CLI's own hierarchy when omitted, which is why omitting
it for a spawned child finds nothing.

`--all` or `--pending` is required (exactly one).

### The two-step read

`logs list` gives you rows and their **part ids**; `logs open --id N`
gives you a part's content. Rows come back as `request_message_user`,
`assistant_response`, `tool_response`.

```bash
objectiveai agents logs list --target "instance=$LEAF,parent=daemon" --all
# {"type":"tool_response", … "parts":[{"id":2422, …}]}

objectiveai agents logs open --id 2422
# {"type":"text","text":"{\"key\":\"probe\",\"value\":\"hello\", …}"}
```

For a tool-driving script agent the interesting row is almost always
the `tool_response` — that is the plugin's actual return value, errors
included. Piping `logs list` through a tiny filter to print
`type` + part ids keeps the output small.

`logs subscribe` waits for NEW rows (`--request`, `--assistant`,
`--tool`) if you want to watch a run live instead of polling.

## Finding an agent you lost the id for

- **`agents tags apply --name <tag> --agent-instance <aih>`** binds a
  name to an instance and — usefully — echoes back the resolved
  `parent_agent_instance_hierarchy` and full `agent_instance_hierarchy`,
  which is a quick way to confirm the parent/instance split.
- **`agents tags lookup --tag <tag>`** resolves it back.
- **`agents instances list --target me`** lists the direct children of
  the caller. **`--target instance=<aih>`** lists a specific node's
  children.
- **`agents instances get --target instance=<aih>`** returns per-agent
  aggregates AND the agent's full definition — handy for confirming
  which agent actually ran. Note it answers for a nonexistent id too
  (with no `agent` field), so presence of the definition is the real
  signal.

## Lifecycle

- **`agents wait --agent-instance <aih> --active|--inactive`** blocks
  until the agent is up / done. `--inactive` returning `"Ok"` means it
  has finished and released its lock.
- **`agents message`** delivers to a running agent, or resumes a
  dormant one via continuation. Same rule as spawn: run it directly.
- **`agents enqueue`** parks a message without spawning or racing
  delivery.
- **`agents queue open|list|delete|deliver`** manages deferred prompts.
- **`agents mcp resources|tools`** queries a LIVE agent's aggregated MCP
  surface — the way to confirm a plugin's tools actually reached an
  agent, and the exact prefixed tool names it sees.

## Gotchas

- **Plugin agents need a laboratory host.** If a spawn fails with
  `no laboratory host is running for this machine/state`, run
  `objectiveai laboratories spawn` first. It is deliberately not
  auto-started.
- **A dev-registered plugin only runs locally**, so the host must be
  the local one.
- Ids are per-run: every spawn mints a new response id, so a target
  from a previous run finds nothing.
