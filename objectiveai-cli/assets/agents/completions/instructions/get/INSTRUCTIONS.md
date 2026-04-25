# Agent Completion — Create Instructions

You are about to run `objectiveai agents completions create standard …`.
Read all of the following before constructing the command.

## Watching progress

If you want to monitor the agent's progress while it streams, pass
`--detach`. The CLI prints a `Logs ID:` line and a PID, then exits
immediately so you can poll the logs while the completion runs in
the background.

**Never use the built-in `Monitor` tool to watch a `create` command.**
The CLI streams to disk through its log writer; nothing meaningful
goes to stdout while the completion is in flight, so `Monitor` will
sit silent and you will draw the wrong conclusion. Poll with the
`logs get` family below instead.

## Reading the logs

Every agent completion writes a *root* log file. Get it with:

```
objectiveai agents completions logs get <logs-id>
```

The root only contains **references** to other logs — it doesn't
inline the message content. The `messages` field is an array of
references, one per message slot the agent produced (assistant turns
and tool responses). To see what the assistant actually wrote you
must follow each reference down to its per-message file:

```
objectiveai agents completions messages logs get <logs-id> <message-index>
```

For multi-turn agent loops with tool use, the continuation chain is
also stored separately:

```
objectiveai agents completions continuations logs get <logs-id>
```

**Walk the references all the way to the bottom.** The root tells
you "there are N messages here"; the per-message files tell you
*what they say* (text content, reasoning, tool_calls, refusals,
finish_reason, usage). Stopping at any intermediate level will
leave you guessing about the agent's behaviour.

## `get` vs `subscribe`

Default to `get`. It reads what is currently on disk and exits —
fast, predictable, idempotent. You can call it as often as you
want with no side effects.

Switch to `subscribe` **only after** you have already issued `get`,
confirmed that the field you need is not yet present, and you
specifically need to block until the next on-disk write. If `get`
already gives you the data, use `get`.

## Use the CLI directly

The CLI has built-in jq filtering on every `logs get` (pass the
expression as the second positional argument) and emits structured
JSON from every command. **Do not pipe through `python`, `bash`,
`jq`, `awk`, `grep`, etc.** unless shell composition is genuinely
required — the CLI does the same shaping in-process, faster, and
without the quoting hazards of escaping a pipeline.
