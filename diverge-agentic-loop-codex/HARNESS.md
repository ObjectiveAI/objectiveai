# The codex harness — settled decisions

The operating rules for this container's implementation, accumulated
as they are settled. Each entry is a decision already made. Add to
this file as more is figured out. The facts about Codex below were
read from its current documentation (the configuration reference,
authentication, sandboxing, approvals and non-interactive pages of
learn.chatgpt.com) and from `codex-rs/exec/src/cli.rs` on `main` on
2026-09-09, against `@openai/codex` 0.153.4, the version the image
pins; nothing here has been run.

## What is built, and what is not

The bootstrap: the crate, the agent vocabulary with its schema, the
server (`POST /register`, `GET /schema`, `POST /enqueue` → `missed`,
`POST /dequeue` → `empty`, `POST /run` → `409` before registration and
`501` after), and the image. The run chunk brings, in the other
containers' shape: cc's three-phase claim; hermes's queue taken at the
turn's end (Codex cannot be steered mid-turn either — a turn is one
`codex exec`, and invoking it again is the delivery); `codex exec
--json` spawned per turn with the config the agent renders to, its
JSONL events (`thread.started`, `turn.started`, `turn.completed`,
`turn.failed`, `item.started`, `item.completed`, `error`; items are
agent messages, reasoning, command executions, file changes, MCP tool
calls, web searches, plan updates) converted into chunks; `codex exec
resume <SESSION_ID>` as the continuation, the session's rollout files
under `CODEX_HOME` harvested into the caller's database as hermes
harvests its files; and the login from the vault.

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
  at `item.completed`, never started. Commands, MCP calls, web
  searches, file changes, collab calls and the todo list start and
  complete; only the todo list is updated, and it completes at the
  turn's end; every started item still open is completed at the
  turn's end too.
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
`sandbox_mode = "danger-full-access"` and `approval_policy = "never"`
(the container is the sandbox; a prompt waits on a user who is not
there); `mcp_servers.diverge = { url = <the container SDK's
mcp_url()> }` — the caller's tools and resources, through the proxy,
the one MCP server; `hide_agent_reasoning = false`. `--skip-git-repo
-check` on the argv (the workspace is whatever the caller mounted).
Never written: `--ephemeral` (the session IS the continuation),
profiles, `notify`, execpolicy rules, `shell_environment_policy`
(the harness renders the process environment itself), `--image` and
`--output-schema` (the wire is text and chunks).

## Auth: a mount first, then the vault, and only both missing refuses

The agent says nothing about how Codex logs in (user ruling
2026-09-09). At each run's start the harness looks, in order:

1. **A mounted `auth.json`** at `$CODEX_HOME/auth.json`. The caller
   put it there, exactly as cc's caller mounts Claude Code's
   credentials; its kind (API key or ChatGPT tokens) is the caller's
   choice and the harness never reads it. Codex refreshes ChatGPT
   tokens in place, on the mount, so the caller's copy stays current
   without any cycle of the harness's. Found, the vault is never
   asked.
2. **The vault's `OPENAI_CODEX_OAUTH`** — the SDK's well-known key,
   the same document Hermes's `providers.openai-codex` entry carries
   — rendered as `$CODEX_HOME/auth.json`. ROTATING: Codex refreshes
   the tokens during use, so the run owes hermes's cycle (`vault.rs`
   there is the model): lock for 300 s refreshed every 100 s on a
   task, get, render, run, read the file back after every turn and at
   the end, set when changed, unlock — every key attempted, the first
   failure reported. The exact shape Codex 0.153.4 expects in
   `auth.json`, versus the vault document's, is a live-run check: the
   harness renders one from the other and never guesses fields it has
   not seen.
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
4. A ChatGPT `auth.json` rendered from the vault document accepted
   headless, and refreshed tokens read back; and a MOUNTED
   `auth.json` refreshed in place on the mount.
5. `wire_api = "responses"` against the Diverge relay.
6. `codex exec resume <SESSION_ID>` across container restarts once
   the rollout files are restored from the database.
