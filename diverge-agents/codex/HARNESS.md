# The codex harness — settled decisions

The operating rules for this container's implementation, accumulated
as they are settled. Each entry is a decision already made. Add to
this file as more is figured out. The facts about Codex below were
read from its current documentation (the configuration reference,
authentication, sandboxing, approvals and non-interactive pages of
learn.chatgpt.com) and from `codex-rs/exec/src/cli.rs` on `main` on
2026-09-09, against `@openai/codex` 0.153.4, the version the image
pins; the login facts (§Auth) from `codex-rs/core/src/auth.rs` and
its credential store at `rust-v0.163.0` on 2026-10-18, checked
against the pinned `0.153.4`; nothing here has been run.

## What is built

The whole container, never run: the agent vocabulary with its schema,
the login method among it (`agent/auth.rs`); the server (`main.rs`,
hermes's); cc's three-phase claim and hermes's queue; the secrets as
the proxy feeds and collects them (`secrets.rs`) and the login made
from the method and the fed value (`auth.rs`); `config.toml` rendered
from the agent (`config.rs`); the thread's rollouts as the
continuation in the caller's database (`continuation.rs`); and
the run (`run/`): `codex exec --json` spawned per turn
(`process.rs`), its events converted into chunks (`convert.rs`), the
queue taken at the turn's end, the rotating secret read back, the
rollouts harvested.

## The run: one `codex exec` per turn

`codex exec --json --dangerously-bypass-approvals-and-sandbox
--skip-git-repo-check [resume <thread_id>] [-i <file>…] -`, the
message's text on stdin (argv caps one argument at 128 KiB on Linux,
and joined queue messages can exceed it) and its images as files under
the temporary directory, one `-i` each, removed when the turn ends;
audio and a binary resource that is not an image refuse the message
(`400`), `CODEX_HOME=/root/.codex` and the login method's variables
in the environment, cwd the harness's (the caller's mounts are wherever
the caller put them), stderr passed through, `kill_on_drop`. The
bypass flag is the source's own for "environments that are externally
sandboxed"; the config carries no `sandbox_mode` or `approval_policy`,
so that decision has one spelling. The first turn of a fresh lineage
runs without `resume`; every turn after resumes the thread
`thread.started` named. A turn ends when stdout closes; the queue is
then taken (empty closes it and ends the run; pending is delivered as
`user` chunks and their blocks concatenated into the next turn's
message), and the next process spawned. Before the first event of the
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
  `history.jsonl` (display history), not `config.toml` (the
  harness's), not `auth.json` (the fed secret's — written before the
  loop and handed back through the collect, never a row).

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

The agent value (`src/agent/`, the request's `arguments`) is what the harness writes into
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
  base_url, env_key = "CODEX_API_KEY", wire_api = "responses" }` and
  `model_provider = "diverge"`. Keyed by the variable the `api_key`
  method sets (below), so a provider REQUIRES `auth.method =
  "api_key"` and the registration refuses any other method beside
  one (`400`, `{kind: "auth"}`). Absent, Codex's own `openai`
  provider, with whatever method the agent names.
- `auth` → nothing in the file: the method's variables on the
  process, or its file under `$CODEX_HOME` (below).

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

## Auth: the agent names the method, the proxy feeds the one secret

The agent names HOW Codex logs in and never with what (user ruling
2026-10-18: every container but cc takes its auth as a secret, and
codex supports every Codex auth mode). `auth` is REQUIRED, tagged
`method`, and each method declares ONE secret by a fixed name the
template must declare; the registration answers that name with its
mode, the proxy feeds its value on `POST /secrets` before every
loop, and the run puts the value where Codex reads it. The four,
from Codex's own credential store (`rust-v0.163.0`, checked against
the pinned `0.153.4`):

| `method` | declares | mode | where it goes |
| --- | --- | --- | --- |
| `api_key` | `OPENAI_API_KEY` | fixed | the variable `CODEX_API_KEY` |
| `access_token` | `CODEX_ACCESS_TOKEN` | fixed | the variable `CODEX_ACCESS_TOKEN` |
| `auth_file` | `CODEX_AUTH` | rotating | `$CODEX_HOME/auth.json`, whole |
| `workload_identity { federation_rule_id, context? }` | `OPENAI_IDENTITY_TOKEN` | fixed | `$CODEX_HOME/identity-token`, named by `OPENAI_IDENTITY_TOKEN_FILE`, with `OPENAI_FEDERATION_RULE_ID` and, when given, `OPENAI_WORKLOAD_IDENTITY_CONTEXT` from the arguments |

The facts the table rests on:

- `OPENAI_API_KEY` in the ENVIRONMENT is not a credential source for
  the built-in `openai` provider: its `env_key` is `None`, and the
  variable only prefills the interactive login. The earlier design,
  which put the key in the environment under that name, logged
  nothing in unless a custom `provider` named it as `env_key`. `CODEX_API_KEY`
  is the credential store's TOP precedence — above `auth.json` — and
  `codex exec` honours it. So the fed `OPENAI_API_KEY` is handed to
  Codex as `CODEX_API_KEY`, and the provider entry's `env_key` is
  `CODEX_API_KEY` too, which is why a provider requires `api_key`.
- `CODEX_ACCESS_TOKEN` carries a personal access token (`at-…`) or
  an Agent Identity JWT, read after `CODEX_API_KEY`.
- `auth.json` (`AuthDotJson`) is `auth_mode?`, `OPENAI_API_KEY`,
  `tokens {id_token, access_token, refresh_token, account_id}`,
  `last_refresh`, `agent_identity?`, `personal_access_token?`,
  `bedrock_api_key?`, `bedrock_access_keys?`; `auth_mode` is
  inferred when absent. Any of its kinds is a valid `CODEX_AUTH`,
  and the harness never reads what kind it is. When a ChatGPT login
  refreshes, Codex rewrites the file IN PLACE (open, truncate, write,
  `0600`): `tokens.{id,access,refresh}_token` and `last_refresh`
  rotate — proactively five minutes before the access token's expiry
  or eight days after `last_refresh`, reactively on a `401`. That is
  why `CODEX_AUTH` is `rotating`: the file is written from the fed
  object before every loop and read back after the loop's last turn
  (`tokio::fs::read` → JSON) into what the collect answers, so the
  next activation finds the refreshed login. The harness's own write
  is a plain write under root's umask; the `0600` is Codex's, on its
  rewrite. A file that cannot be read back, or is not JSON, leaves
  the fed value to answer the collect — the collect must carry every
  name asked. The other three methods hand nothing back.
- Workload identity is selected by `OPENAI_IDENTITY_TOKEN_FILE` +
  `OPENAI_FEDERATION_RULE_ID` (+ optional
  `OPENAI_WORKLOAD_IDENTITY_CONTEXT`); the token is a `fixed` secret
  written to a file under the home because that is how Codex takes
  it.
- `preferred_auth_method` no longer exists, `forced_login_method` is
  unneeded (the method already fixes which login is written), and
  `wire_api` must be `responses`.

The run (`run/mod.rs`) takes a snapshot of the fed values once at
its start — a feed that lands mid-loop is the next loop's — resolves
the method over it (`auth.rs`: the declared name absent, or of the
wrong JSON type, is the run's refusal `{kind: "secrets"}`, naming the
name and never a value), writes the method's file under the home
before `config.toml` and the first turn, and spawns every turn with
`CODEX_HOME` plus the method's variables. The FUSE-mounted
`auth.json` of the earlier design was the workaround for rotation,
and is gone: the rotating secret is the protocol's own cycle. A
ChatGPT login bills the caller's subscription; an API key bills the
platform at API rates — Codex's docs keep the two apart, and so does
the method. The proxy is touched only at the run, never at startup.

Nothing about credentials rides the agent value, the schema or a row:
the method does, the value is the secret's, and `auth.json` goes
back through the collect. No value of a secret is ever formatted
into an error or a log — names only.

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
4. A `CODEX_AUTH` written to `$CODEX_HOME/auth.json` before the
   loop, accepted headless, rewritten in place by Codex on a ChatGPT
   refresh, and collected after the loop with the rotated tokens in
   it; and `CODEX_API_KEY` alone logging `codex exec` in with no
   `auth.json` at all.
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
