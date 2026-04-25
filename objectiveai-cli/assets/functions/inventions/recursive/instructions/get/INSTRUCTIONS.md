# Function Invention (Recursive) — Create Instructions

You are about to run `objectiveai functions inventions recursive create …`.
Read all of the following before constructing the command.

## Watching progress

Recursive invention can produce many sub-inventions, each with many
agent turns (essay → input schema → essay tasks → tasks →
description → optional retries on validation failure). Pass `--detach`
to monitor in the background; the CLI prints a `Logs ID:` line and a
PID, then exits so you can poll the logs while invention continues.

**Never use the built-in `Monitor` tool to watch a `create` command.**
The CLI streams to disk through its log writer; nothing meaningful
goes to stdout while invention is in flight, so `Monitor` will sit
silent and you will draw the wrong conclusion. Poll with the
`logs get` family below.

## Reading the logs

The recursive root log:

```
objectiveai functions inventions recursive logs get <logs-id>
```

contains only **references** — `inventions` is an array, one entry
per function the recursive process is inventing (or has invented).
Each entry is a reference to a per-invention log. Follow the
reference with:

```
objectiveai functions inventions logs get <invention-id>
```

Inside the per-invention log, `completions` is itself an array of
**references** to agent-completion logs (`agtcpl-…`). Each agent
completion's `messages` array is references too — descend with:

```
objectiveai agents completions logs get <agtcpl-id>
objectiveai agents completions messages logs get <agtcpl-id> <message-index>
```

So the full descent is:

```
recursive root  →  invention  →  agent completion  →  message
```

**Walk the references all the way to the bottom**, level by level.
The root tells you "there are N inventions"; the invention log tells
you "there were K agent completions during this invention's steps";
the agent completion's per-message files tell you *what the agent
actually wrote* (essay text, schema JSON, task definitions, retry
prompts). Stopping at any intermediate level will leave you with
state objects and references but no actual content.

## `get` vs `subscribe`

Default to `get`. It reads what is currently on disk and exits —
fast, predictable, idempotent. You can call it as often as you want
with no side effects.

Switch to `subscribe` **only after** you have already issued `get`,
confirmed that the specific field you need is not yet present, and
you actually need to block until the next on-disk write. If `get`
already gives you the data, use `get`.

## Use the CLI directly

The CLI has built-in jq filtering on every `logs get` (pass the
expression as the second positional argument) and emits structured
JSON from every command. **Do not pipe through `python`, `bash`,
`jq`, `awk`, `grep`, etc.** unless shell composition is genuinely
required — the CLI does the same shaping in-process, faster, and
without the quoting hazards of escaping a pipeline.
