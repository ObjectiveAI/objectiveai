# Function Execution — Create Instructions

You are about to run `objectiveai functions executions create standard …`
or `… create swiss-system …`. Read all of the following before
constructing the command.

## Watching progress

Function executions are deep — a branch function with split mode and
a Swiss strategy can fan out into hundreds of concurrent agent calls.
Pass `--detach` to monitor in the background; the CLI prints a
`Logs ID:` line and a PID, then exits so you can poll the logs while
execution continues.

**Never use the built-in `Monitor` tool to watch a `create` command.**
The CLI streams to disk through its log writer; nothing meaningful
goes to stdout while the execution is in flight, so `Monitor` will
sit silent and you will draw the wrong conclusion. Poll with the
`logs get` family below.

## Reading the logs

The root log:

```
objectiveai functions executions logs get <logs-id>
```

contains only **references** — `tasks` is an array, and every entry
is a reference to either:

- A **vector completion** task. Follow the reference with:

  ```
  objectiveai vector completions logs get <vctcpl-id>
  ```

  Inside, `completions` is itself an array of **references** to
  per-agent agent-completion logs (`agtcpl-…`). Each agent completion's
  `messages` array is references too — descend with:

  ```
  objectiveai agents completions logs get <agtcpl-id>
  objectiveai agents completions messages logs get <agtcpl-id> <message-index>
  ```

- A **nested function execution** (split mode wraps the per-element
  execution; Swiss wraps each pool/round; branch sub-functions wrap
  their leaves). Follow the reference back into:

  ```
  objectiveai functions executions logs get <fnexec-id>
  ```

  …and apply this same procedure recursively. Each nested wrapper
  carries `split_index` / `swiss_pool_index` / `swiss_round` /
  `task_path` so you can locate where in the tree you are.

**Walk the references all the way to the bottom**, recursively
through every level of `tasks`. The root only carries the references
and aggregate scores; the actual *content* (which response key each
agent picked, the reasoning, the tool calls) lives in the leaf
agent-completion message files. Don't stop at the vector completion
or the function-execution wrapper — descend until you reach the
per-message files at the leaves.

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
