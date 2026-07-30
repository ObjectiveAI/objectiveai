---
name: agent-control
description: Drive and inspect ObjectiveAI agents from the CLI — spawn, message, wait, tags, and above all the persisted `agents logs` tier for reading exactly what an agent did. Use when running an agent, or when an agent has run and you need its tool calls and results.
---

# Controlling agents

## Terms — instance vs. hierarchy

Two different things, and conflating them is the single most expensive
mistake here:

- **instance** — the agent's own identity. ONE segment, no slashes:
  `3Bk4hBIOrMDdNaDL3atd1b-F1LTN4MMGSB1wpHXF`. This is what
  `--agent-instance` and `--target instance=` want.
- **agent instance hierarchy (AIH)** — the FULL joined lineage:
  every ancestor plus the instance, slash-separated:
  `cli/daemon/3Bk4hBIOrMDdNaDL3atd1b-F1LTN4MMGSB1wpHXF`. This is what
  log rows report in their `agent_instance_hierarchy` field.

`agents tags apply` shows the decomposition outright, which makes it a
good way to confirm a split:

```json
{"agent_instance": "54qv…-0exi…",
 "parent_agent_instance_hierarchy": "cli/daemon",
 "agent_instance_hierarchy": "cli/daemon/54qv…-0exi…"}
```

**Most commands take the INSTANCE ALONE, never the joined hierarchy.**
Ancestors go separately in `parent=`, and the CLI prepends its own AIH
to whatever you pass there — so `parent=daemon` resolves to
`cli/daemon` above.

## Run the command directly. Do NOT redirect to a file.

`agents spawn` and `agents message` print **one short line** — a
parent-relative path in quotes, nothing else. Backgrounding them to a log
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

**A spawn prints a PATH — `daemon/<instance>` — not an instance. Split
it: the last segment is the INSTANCE, everything before it is the
PARENT. Passing the joined string as `instance=` is the mistake.**

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

- **`agents tags apply --name <tag> --agent-instance <instance>`** binds
  a name to an instance and — usefully — echoes back the resolved
  `agent_instance`, `parent_agent_instance_hierarchy` and full
  `agent_instance_hierarchy`. That decomposition is the quickest way to
  confirm a split when a lookup is coming back empty.
- **`agents tags lookup --tag <tag>`** resolves it back.
- **`agents instances list --target me`** lists the direct children of
  the caller. **`--target instance=<instance>[,parent=<ancestors>]`**
  lists a specific node's children.
- **`agents instances get --target instance=<instance>`** returns per-agent
  aggregates AND the agent's full definition — handy for confirming
  which agent actually ran. Note it answers for a nonexistent id too
  (with no `agent` field), so presence of the definition is the real
  signal.

## Lifecycle

- **`agents wait --agent-instance <instance> --active|--inactive`**
  blocks until the agent is up / done. Beware `--inactive` as an
  existence check: a nonexistent agent is trivially inactive, so `"Ok"`
  proves nothing about resolution on its own.
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
