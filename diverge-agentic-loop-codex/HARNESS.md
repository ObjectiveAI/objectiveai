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
- `login` → `forced_login_method = "api" | "chatgpt"`; absent is
  `api`. Decides the vault key (below).
- `provider` → `model_providers.diverge = { name = "diverge",
  base_url, env_key = "OPENAI_API_KEY", wire_api = "responses" }` and
  `model_provider = "diverge"`. Requires an API-key login; both a
  provider and a ChatGPT login is a contradiction the run refuses.
  Absent, Codex's own `openai` provider.

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

## Auth comes from the vault, by `login`

- `api_key`: `OPENAI_API_KEY` is a STATIC vault secret, read once per
  run and put in the process environment, which is where Codex's
  `env_key` looks. A vault without it refuses the run.
- `chatgpt`: `OPENAI_CODEX_OAUTH` — the SDK's well-known key, the
  same document Hermes's `providers.openai-codex` entry carries — is
  ROTATING: Codex refreshes the tokens during use. The run owes
  hermes's cycle (`vault.rs` there is the model): lock for 300 s
  refreshed every 100 s on a task, get, render as
  `$CODEX_HOME/auth.json`, run, read the file back after every turn
  and at the end, set when changed, unlock — every key attempted, the
  first failure reported. The exact shape Codex 0.153.4 expects in
  `auth.json` for a ChatGPT login, versus the shape the vault document
  carries, is a live-run check: the harness renders one from the
  other and never guesses fields it has not seen.

Nothing about credentials rides the agent value, the schema, a row or
a mount.

## What a live run has to prove

1. The `--json` event vocabulary at 0.153.4, item by item, before the
   converter is written — the docs name the event families, not the
   fields.
2. `danger-full-access` with `approval_policy = "never"` running
   inside the container as root without a Landlock or seccomp
   complaint, and with the MCP server reachable on the loopback.
3. `mcp_servers.diverge` with a bare `url` (Streamable HTTP, no
   bearer) connecting to the proxy's inside port, listing the
   caller's tools, and reading resources.
4. A ChatGPT `auth.json` rendered from the vault document accepted
   headless, and refreshed tokens read back.
5. `wire_api = "responses"` against the Diverge relay.
6. `codex exec resume <SESSION_ID>` across container restarts once
   the rollout files are restored from the database.
