# The codex harness — settled decisions

The operating rules for this container's implementation, accumulated
as they are settled. Each entry is a decision already made. Add to
this file as more is figured out. The facts about Codex below were
read from its current documentation (the configuration reference,
authentication, sandboxing, approvals and non-interactive pages of
learn.chatgpt.com) and from `codex-rs/exec/src/cli.rs` on `main` on
2026-09-09, against `@openai/codex` 0.153.4, the version the image
pins; nothing here has been run.

## What is built

The whole container, never run: the agent vocabulary with its schema;
the server (`main.rs`, hermes's); cc's three-phase claim and hermes's
queue; the login from a mount or the vault (`auth.rs`); `config.toml`
rendered from the agent (`config.rs`); the thread's rollouts as the
continuation in the caller's database (`continuation.rs`); and the
run (`run/`): `codex exec --json` spawned per turn (`process.rs`), its
events converted into chunks (`convert.rs`), the queue taken at the
turn's end, the login read back, the rollouts harvested.

## The run: one `codex exec` per turn

`codex exec --json --dangerously-bypass-approvals-and-sandbox
--skip-git-repo-check [resume <thread_id>] -`, the prompt on stdin
(argv caps one argument at 128 KiB on Linux, and joined queue prompts
can exceed it), `CODEX_HOME=/root/.codex` and the login's variable in
the environment, cwd the harness's (the caller's mounts are wherever
the caller put them), stderr passed through, `kill_on_drop`. The
bypass flag is the source's own for "environments that are externally
sandboxed"; the config carries no `sandbox_mode` or `approval_policy`,
so that decision has one spelling. The first turn of a fresh lineage
runs without `resume`; every turn after resumes the thread
`thread.started` named. A turn ends when stdout closes; the queue is
then taken (empty closes it and ends the run; pending is delivered as
`user` chunks and joined with a blank line into the next turn's
prompt), and the next process spawned. Before the first event of the
first process a failure is the request's own (`500`); after, a fatal
notification, and the run still harvests.

Rules settled with it:

- An interrupted turn (stdout closed after `turn.started` with no
  terminal event) is a fatal notification `{kind: "interrupted"}`.
  A non-zero exit after a terminal event is a non-fatal notification.
  A line that is not an event is a non-fatal notification after the
  first event, the request's own failure before it.
- Tool-call ids are MINTED here (uuid v4), mapped from the item id
  for the process's life: Codex's `item_<n>` ids restart every
  process, and the stream needs an id unique across the run. A
  completed item that never started (agent messages and reasoning
  always; anything reconciled at the turn's end) gets a fresh id and,
  for a tool, a call and a response back to back.
- The converter, per item: `agent_message` → `assistant_text_content`
  whole; `reasoning` → `assistant_reasoning` whole;
  `command_execution` → call `command_execution` `{command}` /
  response one text block of `aggregated_output`, `is_error` on
  failed or declined; `file_change` → call `file_change` `{changes}`
  / response one text line per change (`<kind> <path>`), `is_error`
  on failed; `mcp_tool_call` → call named the TOOL (the server is
  `diverge`; another server is prefixed `<server>/`) with the
  arguments verbatim / response the MCP result verbatim — content
  blocks parsed into rmcp's own, a block that will not parse kept as
  a text block of its JSON — `structured_content`, `_meta`, `is_error`
  on failed, an `error` without a result as one text block of its
  message; `web_search` → call `web_search` `{query, action}` /
  response the action as text; `todo_list` (started, updated,
  completed) → non-fatal notification `{kind: "todo_list", items}`;
  `collab_tool_call` → non-fatal notification `{kind: "collab", ...}`;
  the `error` item → non-fatal `{kind: "codex", error}`. The `error`
  EVENT → non-fatal `{kind: "codex", error}`; `turn.failed` → fatal
  `{kind: "turn", error}`; `turn.completed` → `usage`.
- Usage is a DELTA against the thread's baseline: `turn.completed`
  reports the thread's cumulative count; the chunk is cumulative minus
  the last cumulative seen (each counter clamped at zero), and when
  the cumulative is smaller than the baseline the process is counting
  from zero and the delta is the cumulative itself. `prompt_tokens` =
  input, `completion_tokens` = output (reasoning tokens are inside
  output), `total` their sum. The baseline is the thread row's.

## The continuation is the thread's rollouts

`codex exec resume <thread_id>` finds a thread through its SQLite
index first and falls back to scanning the rollout files, so the
files are the continuation and the index is Codex's to rebuild. Two
tables in the caller's database:

```sql
CREATE TABLE IF NOT EXISTS codex_thread (
    id smallint PRIMARY KEY CHECK (id = 1),
    thread_id text NOT NULL,
    usage jsonb NOT NULL)
CREATE TABLE IF NOT EXISTS codex_files (
    path text PRIMARY KEY, content bytea NOT NULL)
```

- The thread row is read once per program and cached (cc's rule);
  the files are restored under `$CODEX_HOME` once per program and
  never again (hermes's rule) — Codex appends to them itself, and the
  disk is the authority from then on. Paths are validated (relative,
  no `..`) before a write.
- Every run harvests at its end, whenever a thread is known — the
  fatal end included, since the files are the truth of what happened:
  every file under `sessions/` and `archived_sessions/` whose name
  carries the thread id, read as BYTES (rollouts may be `.zst` once
  compressed), replacing the rows whole in one transaction beside the
  thread row.
- Nothing else travels: not the state database (an index), not
  `history.jsonl` (display history), not `config.toml` and
  `auth.json` (the harness's and the vault's).

## The stream is `codex exec --json`, and `response/` is its vocabulary

Read from `codex-rs/exec/src/exec_events.rs` and
`event_processor_with_jsonl_output.rs` at tag `rust-v0.153.4`
(2026-09-09); typed, strict, deserialize-only in `src/response/`,
cc's shape (marker `type` fields, untagged unions, no catch-all — the
pin closes the union; the one open tail is the source's own
`WebSearchAction::Other`).

- One JSON object per line, and NOTHING else on stdout: the final
  message goes to a file only with `-o`, and a serialization failure
  is itself written as an `error` event.
- Eight events: `thread.started {thread_id}` first, on a fresh and a
  resumed thread alike; `turn.started`; `item.started` /
  `item.updated` / `item.completed {item}`; `turn.completed {usage}`
  or `turn.failed {error}`; `error {message}`. One process is ONE
  turn: completion initiates shutdown, and the next turn is `codex
  exec resume <thread_id> --json`.
- Nine items, `{id, type, ...}`: `agent_message`, `reasoning`,
  `command_execution`, `file_change`, `mcp_tool_call`,
  `collab_tool_call`, `web_search`, `todo_list`, `error`. Ids are
  `item_<n>`, minted per process.
- NO DELTAS. Agent messages and reasoning summaries arrive whole, once,
  at `item.completed`, never started (a reasoning summary is its
  lines joined with `\n`; a blank one produces no item). Commands,
  MCP calls, web searches, collab calls and the todo list start and
  complete; a file change is documented as completed only, though
  the mapping would pass a start through; only the todo list is
  updated, and it completes at the turn's end; every started item
  still open is completed at the turn's end too.
- An INTERRUPTED turn ends with NEITHER `turn.completed` nor
  `turn.failed`: the processor writes nothing and shuts down. Stdout
  closing after `turn.started` with no terminal event is an
  interruption, and the converter treats it as one.
- The items are a projection of the core's: four collab tools and an
  interrupted collab call produce no item, `resume_agent` is reported
  as `wait`, and a declined patch is reported as failed.
- Non-fatal news — warnings, config warnings, deprecations, a model
  reroute — is an `item.completed` with an `error` ITEM. The `error`
  EVENT is critical but does not end the turn by itself; `turn.failed`
  follows, carrying the turn's own error or the last critical one.
- `turn.completed.usage` is the thread's CUMULATIVE usage
  (`usage_from_last_total`), not the turn's: the converter bills the
  difference from what the previous turn reported.
- `mcp_tool_call.result.content` is kept as raw JSON blocks, as the
  source keeps it; the converter turns them into rmcp's
  `CallToolResult` for the `tool_response` chunk, byte-faithfully.

## The image (see also the Containerfile header)

`debian:sid` by digest, cc's; node and npm, python, git, curl. Codex
is PRE-INSTALLED at build — `npm install -g @openai/codex@0.153.4` —
because it is Apache-2.0 and nothing forbids shipping it; the pin
lives in the Containerfile and nowhere else. The container proxy is
NOT in the image: the host injects it at runtime, and it may not be
up until a request comes. The harness is the entrypoint.

## The agent renders to config.toml and argv

The agent value (`src/agent/`) is what the harness writes into
`$CODEX_HOME/config.toml` and passes on `codex exec`'s argv, and
nothing else:

- `model` → `model` (also `--model`).
- `effort` → `model_reasoning_effort = minimal | low | medium | high
  | xhigh` (the SDK's old reference had `max` and lacked `minimal`;
  Codex has neither `max` nor a sixth rung).
- `reasoning_summary` → `model_reasoning_summary = auto | concise |
  detailed | none`; what the reasoning chunks carry.
- `verbosity` → `model_verbosity = low | medium | high`.
- `web_search` → `web_search = disabled | cached | indexed | live`;
  ABSENT IS `disabled` (nothing is on by omission). A search is
  Codex's own, never one of the caller's tool calls on the stream.
- `provider` → `model_providers.diverge = { name = "diverge",
  base_url, env_key = "OPENAI_API_KEY", wire_api = "responses" }` and
  `model_provider = "diverge"`. Keyed by the `OPENAI_API_KEY` the run
  found (below); beside a ChatGPT login it fails as itself, in Codex's
  words. Absent, Codex's own `openai` provider.

Every absent `Option` leaves its key unset, so Codex applies its own
default for the model — except `web_search`, above.

Always written, no switch (the reasons are in `agent/mod.rs`):
`mcp_servers.diverge = { url = <the container SDK's mcp_url()> }` —
the caller's tools and resources, through the proxy, the one MCP
server; `hide_agent_reasoning = false`. On the argv, not in the
config: `--dangerously-bypass-approvals-and-sandbox` (the container
is the sandbox; a prompt waits on a user who is not there) and
`--skip-git-repo-check` (the workspace is whatever the caller
mounted). `config.toml` is written before every run at
`$CODEX_HOME/config.toml`, the home made as needed. Never written:
`--ephemeral` (the thread IS the continuation), profiles, `notify`,
execpolicy rules, `shell_environment_policy` (the harness renders the
process environment itself), `--image` and `--output-schema` (the
wire is text and chunks).

## Auth: a mount first, then the vault, and only both missing refuses

The agent says nothing about how Codex logs in (user ruling
2026-09-09). At each run's start the harness looks, in order:

1. **A mounted `auth.json`** at `$CODEX_HOME/auth.json`. The caller
   put it there, exactly as cc's caller mounts Claude Code's
   credentials; its kind (API key or ChatGPT tokens) is the caller's
   choice and the harness never reads it. Codex refreshes ChatGPT
   tokens in place, on the mount, so the caller's copy stays current
   without any cycle of the harness's. Found, the vault is never
   asked. A file THIS PROGRAM wrote from the vault in an earlier run
   is not a mount (a static remembers), so the vault stays
   authoritative across runs.
2. **The vault's `OPENAI_CODEX_OAUTH`** — the SDK's well-known key,
   the same document Hermes's `providers.openai-codex` entry carries
   — written VERBATIM as `$CODEX_HOME/auth.json`: the document is
   Codex's own file, and no field is ever rendered or guessed. Read
   WITHOUT a lock and never written back (user ruling 2026-09-09):
   the document does not rotate in the container — it carries the
   access token the caller refreshes on their side — so there is no
   cycle to owe. That the vault's document is exactly the `auth.json`
   Codex 0.153.4 accepts is a live-run check.
3. **The vault's `OPENAI_API_KEY`**: a STATIC secret, read once per
   run and put in the process environment, where Codex's `env_key`
   looks. No lock, no set.

None of the three: the run is refused, naming all three places. The
OAuth document outranks the API key in the vault because the key is
a generic secret other images share (Eliza reads the same one), so
its presence says nothing about Codex, while the document is Codex's
own and its presence is intent. A ChatGPT login bills the caller's
subscription; an API key bills the platform at API rates — Codex's
docs keep the two systems apart, and so does this order. The vault is
touched only at the run, never at startup: the proxy may not be up
before a request.

Nothing about credentials rides the agent value, the schema or a row.

## What a live run has to prove

1. The `--json` event vocabulary as typed in `src/response/`, line by
   line against a real run at 0.153.4 — the types are from the
   source, and a strict parser dies on the first surprise.
2. `danger-full-access` with `approval_policy = "never"` running
   inside the container as root without a Landlock or seccomp
   complaint, and with the MCP server reachable on the loopback.
3. `mcp_servers.diverge` with a bare `url` (Streamable HTTP, no
   bearer) connecting to the proxy's inside port, listing the
   caller's tools, and reading resources.
4. The vault's document written verbatim as `auth.json` accepted
   headless; and a MOUNTED `auth.json` refreshed in place on the
   mount.
5. `wire_api = "responses"` against the Diverge relay.
6. `codex exec resume <SESSION_ID>` across container restarts once
   the rollout files are restored from the database, with the SQLite
   index rebuilt from them, and `.zst` rollouts after compression.
7. `-` on stdin under `exec resume` reading the whole prompt, as it
   does under plain `exec`.
8. Whether a resumed process's `turn.completed.usage` counts from the
   thread's beginning or from zero — the delta logic tolerates both,
   but the bill is only right if one of them is what happens.
9. `mcp_servers.diverge` with a bare `url` connecting to the proxy's
   inside port and the model calling the caller's tools through it.
