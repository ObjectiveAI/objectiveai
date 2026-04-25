# Laboratory Execution — Create Instructions

You are about to run `objectiveai laboratories executions create …`.
Read all of the following before constructing the command.

## Watching progress

Laboratory executions are slow: each builder agent runs inside its
own Docker container, builders run concurrently, and (when an
evaluation agent is configured) every builder's output is graded by
a follow-up agent call. Pass `--detach` to monitor in the
background; the CLI prints a `Logs ID:` line and a PID, then exits
so you can poll the logs while builders are still grinding.

**Never use the built-in `Monitor` tool to watch a `create` command.**
The CLI streams to disk through its log writer; nothing meaningful
goes to stdout while the execution is in flight, so `Monitor` will
sit silent and you will draw the wrong conclusion. Poll with the
`logs get` family below.

## Reading the logs

The root log:

```
objectiveai laboratories executions logs get <logs-id>
```

contains only **references** — both the `builders` array (one entry
per builder agent) and the `evaluations` array (one entry per
evaluated builder) are lists of references to per-agent agent-completion
logs (`agtcpl-…`). Follow each one with:

```
objectiveai agents completions logs get <agtcpl-id>
```

…and inside that log, `messages` is itself a list of references to
per-message files. Descend with:

```
objectiveai agents completions messages logs get <agtcpl-id> <message-index>
```

**Walk the references all the way to the bottom.** The root only
carries the references and aggregate scores. The actual *content*
(what code each builder wrote inside its Docker sandbox, what each
evaluator concluded, retry prompts, finish reasons) lives in the
leaf agent-completion message files. Don't stop at the root or even
the per-builder agent-completion log — descend until you reach the
per-message files at the bottom.

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
