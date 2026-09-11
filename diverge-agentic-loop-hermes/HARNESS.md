# The hermes harness — settled decisions

The operating rules for this container's implementation, accumulated
as they are settled. Each entry is a decision already made, with its
evidence in `reports/` where one exists. Add to this file as more is
figured out.

## Auth is handled on the filesystem, from the request

Nothing about credentials arrives ambiently: the request's agent
carries the static ones as arguments (provider structures, toolset
structures), the ROTATING ones live in the caller's vault under the
SDK's well-known keys, and the harness APPLIES all of them before
`hermes gateway` starts —

- canonical env vars in the gateway's PROCESS environment (the
  per-provider and per-toolset docs in the agent module name each
  one);
- `$HERMES_HOME/auth.json` entries for the OAuth providers (nous,
  openai-codex, minimax-oauth) and spotify — the vault document
  written verbatim as `providers.<name>`;
- qwen-oauth's document at the REAL `$HOME`
  (`~/.qwen/oauth_creds.json`) — and NO `providers.qwen-oauth`
  marker: that entry serves the setup wizard's credential check
  only; the gateway reads the CLI file directly;
- vertex's service-account JSON to a file + `VERTEX_CREDENTIALS_PATH`;
- `model.base_url` + `model.api_key` in config.yaml for `custom`
  (Hermes reads the key only beside a configured URL; the env var
  `CUSTOM_BASE_URL` is set too, and is what it resolves first).

Rules that make this work (`provider-auth.md`, `oauth-resources.md`):

- The image ships NO `~/.hermes/.env` — that file SHADOWS the
  process environment when present.
- Provider selection is `model.provider` in config.yaml, NEVER
  auth.json's `active_provider` (priority 6 of 8; a stray
  `OPENAI_API_KEY` would outrank it). Omit `suppressed_sources`
  when writing auth.json.
- Where two toolsets name the same env var (`FAL_KEY`), the values
  must agree; disagreement is the caller's contradiction.
- Rotating OAuth state is a VAULT DOCUMENT under a fixed key
  (`NOUS_OAUTH`, `OPENAI_CODEX_OAUTH`, `MINIMAX_OAUTH`, `QWEN_OAUTH`,
  `SPOTIFY_OAUTH` — the SDK's `vault::keys`, shared by any image
  that speaks the same login). The agent names no field for it:
  choosing the provider or toolset is the whole ask. Every run owes
  the cycle, in `vault.rs`: LOCK the key (TTL 300 s, re-locked every
  100 s on a task for the run's life), GET the document, write it,
  run, read it back as Hermes left it, SET it, UNLOCK. A key the
  vault does not hold, or holds as something other than a JSON
  object, refuses the run before the gateway starts. A document no
  longer on disk at the end (Hermes quarantined the entry) is not
  set — the caller keeping its copy beats a lost login — and the key
  is still unlocked. A run abandoned mid-way stops refreshing and
  the locks lapse by their TTL.
- The alternative to the vault cycle is a FUSE DIRECTORY mount
  (`fuse_directory_mounts` on the container request): the caller
  serves `$HERMES_HOME` — or the directory holding `auth.json` and
  the qwen file — live, and every refresh Hermes writes lands with
  the caller as it happens. A DIRECTORY, not a file mount: Hermes
  saves its auth store by writing a temporary beside it and renaming
  over it (`hermes_cli/auth.py`, `_save_auth_store`, an atomic
  `os.replace`), and the kernel refuses a rename onto a single-file
  mount point. Which the caller chooses is the request's; the
  harness's cycle runs only for keys the vault holds.

## The `filesystem` module lays it down, and reads the documents back

Three entry points: `filesystem::plan` reads the agent into the
`Plan` — the environment, the config's inputs, and which vault
documents the run needs; `filesystem::prepare` writes the plan down
with the documents in hand; `filesystem::read_back` reads each
document off disk after the gateway has exited (the `auth.json`
entry re-serialized, the Qwen file verbatim — a document no longer
there is `None`), for the vault. The continuation is not the
filesystem module's to move any more: `continuation.rs` restores it
from the database once and harvests it at every run's end.

`filesystem::prepare` turns the plan into the gateway's process
environment (returned to the spawner — the request carries no
environment of its own; the agent's typed fields and the vault are
the whole source), `config.yaml`, and the credential files, in one
pass:

- `config.yaml` is written as JSON (JSON is YAML): `model.provider`
  (the agent marker's id IS Hermes's) + `model.default`,
  `model.base_url` + `model.api_key` for `custom` only,
  `security.protected_instruction_files: false`, the backend pins
  asked for (`web.search_backend` / `web.extract_backend` by plugin
  name — `tavily`, `exa`, `parallel`, `keenable`, `brave-free`,
  `searxng`, `firecrawl`; `tts.provider: elevenlabs` when keyed;
  `image_gen.provider` / `video_gen.provider: fal`), and the MCP
  proxy entry `diverge` → the container SDK's `mcp_url()` (the
  proxy's inside port, `/mcp`) with `trust: full`, elicitation AND
  sampling disabled.
- Toolset exposure is the explicit list `platform_toolsets.api_server`
  (the only deterministic form; there is no `disabled_toolsets`
  config key). `true` or a structure present → in; `false`, absent,
  or a structure absent → out. The plain switches are `Option<bool>`
  (absent = off), the argument-taking ones `Option` structures, and
  nothing is on by omission: Hermes's own API-server default — web,
  browser, terminal, file, code_execution, vision, todo, memory,
  session_search, and image_gen (hidden without a FAL key, which only
  the structure brings) — is never consulted.
  `skills` in exactly when something is on disk under the external
  skills path, out otherwise — and skills come from MOUNTS at
  `/root/.hermes/external-skills/` (one directory per skill, its
  `SKILL.md` inside), which the config names as `skills.external_dirs`:
  Hermes discovers them recursively and views them, never writes
  (read-only to the curator, the usage tracker, the hub). Its own
  `skills/` is off limits for mounts — bundled skills sync into it
  at startup and the bookkeeping lives there. The request carries no
  mount list any more, so the disk is probed: a non-empty directory
  is skills mounted. delegation, cronjob, clarify, computer_use,
  discord, yuanbao, context_engine, stt never.
- `HERMES_HOME=/root/.hermes` is set explicitly, pinning the geometry
  the files were laid down under.
- The API server needs a usable bearer: `API_SERVER_KEY` is minted
  per run (64 hex), `API_SERVER_HOST=127.0.0.1`,
  `API_SERVER_PORT=8642`; `API_SERVER_ENABLED` is inert and unset.
- Each credential file is written exactly once — `auth.json` as
  `{"version": 1, "providers": {...}}` with the documents verbatim,
  the Qwen file, the vertex file (`VERTEX_CREDENTIALS_PATH` names
  it).
- One variable, one value: `FAL_KEY` (image_gen vs video_gen) and
  `XAI_API_KEY` (the xai provider vs x_search) must agree, or the
  request contradicts itself and is refused.

## Use yolo mode — the container is the sandbox

Hermes's permission layer protects a user's real machine; this
container has none. Three switches make every approval prompt
unreachable (there is deliberately no single master switch):

1. `HERMES_YOLO_MODE=1` in the gateway's process env — read once at
   import, frozen; silences the terminal guard, the execute_code
   gate, and the plugin-escalation/ssh-config gate.
2. `security.protected_instruction_files: false` in config.yaml —
   that gate ignores yolo by upstream design.
3. `elicitation: {enabled: false}` on the proxy's MCP server entry
   (plain MCP calls already never prompt at the default
   `trust: full`).

Residue: a small hardline blocklist still refuses a handful of
catastrophic commands outright, promptless and unswitchable —
harmless here; the model reads the refusal and routes around it.

Even so, the harness KEEPS reading `approval.request` /
`approval.responded` (the response module types them): if a prompt
ever fires anyway, answer it instantly at
`POST /v1/runs/{id}/approval` instead of eating the 300-second
fail-closed stall it otherwise costs.

## The wire is /v1/runs, and the stream is not the tool channel

- Drive the gateway API server: `POST /v1/runs`, events at
  `GET /v1/runs/{id}/events` — `run::raw::run` is that transport,
  yielding each data frame as the `response::Event` it parses to;
  keepalives and the closing sentinel are SSE comments the parser
  drops, and the subscription is single and never retried.
  `/v1/runs` LOADS NO HISTORY: `session_id` only names the row the
  turn records into, and `conversation_history` is role + content
  TEXT, nothing richer (the chat-completions handler does the load
  itself; the runs handler leaves it to the caller). So
  `filesystem::history` mirrors Hermes's loader reduced to that —
  the session's `user`/`assistant` rows, `active = 1`, in insertion
  order, JSON-sentinel content flattened to its text parts — and
  `run::raw::run` reads it for the named session and sends it. A
  resumed run sees what was said, not what was called; tool rows
  sent as text would be dropped by the loop's repair anyway. The stream's full contract is
  `run-event-stream.md` and the `response` module: 12 event types
  discriminated by the payload's own `event` key; `: keepalive`
  and `: stream closed` are SSE COMMENTS; EOF must always terminate
  (a mid-stream failure closes the socket with no sentinel); the
  queue buffers from POST time but serves exactly ONE subscriber,
  destructively, with a 300s unsubscribed TTL.
- Tool responses are NEVER on that stream
  (`gateway-tool-response.md`) — the container emits its
  `tool_response` chunks from its own MCP proxy's view, the only
  byte-faithful one. The gateway events
  supply timing, reasoning glimpses, deltas and the bill.
- Hermes cannot be steered mid-turn, and the proxy holds no queue:
  the container holds its own (`queue.rs`, openrouter's), and takes
  it at each turn's end — every pending message answered
  `delivered`, yielded as a `user` chunk, and joined with a blank
  line between (openrouter's join) into the next turn's input on the
  same session. Invoking Hermes again IS the delivery. An empty queue
  at a turn's end closes it in the same lock hold and ends the run;
  later enqueues are `missed`.

## Toolsets are applied, not passed through

The agent's toolset structures become env vars + config keys the
harness writes: enable/disable via Hermes's toolset config, backend
pins where a slot names one (`web.search_backend`,
`web.extract_backend`, `tts.provider: elevenlabs` when the key is
present). Deliberately out of the vocabulary and to be LEFT at
Hermes defaults or off: context_engine, yuanbao, skills-as-a-switch
(mounts decide it), computer_use, stt, clarify, discord, cronjob, spotify's
managed-gateway sibling (the rotating Nous tool gateway stays
unsupported). DELEGATION IS DISABLED: the harness turns the
`delegation` toolset off in Hermes's toolset config, and the
vocabulary carries no switch for it. On the API server every
delegation is background — the run ends before its children, and
their results wake the session as a turn no stream sees
(`reports/subagent-correlation.md`,
`reports/delegation-observability.md`). It could be supported
through end-of-turn webhooks; the decision is not to.

## The continuation is state.db and the two memory files

Three files under the fixed home `/root/.hermes` (no `HERMES_HOME`
override): `state.db` whole — the session store, schema v26 and
moving, its gateway-only tables empty here — plus
`memories/MEMORY.md` and `memories/USER.md`. Nothing else travels:
configuration is rendered from the request, `auth.json` entries are
vault documents, caches regenerate, skill writing is unsupported. The
`filesystem::continuation` module is the shape; `continuation.rs` is
where it lives; the rules:

- The continuation is NEVER held whole in memory: the harvest
  streams each file out in pieces of at most 2 MiB, each piece behind
  one tag byte naming its file (`0` state.db, `1` MEMORY.md, `2`
  USER.md), and each piece is ONE ROW of the caller's table
  `continuation (seq integer PRIMARY KEY, frame bytea NOT NULL)`, in
  order, written in one transaction that replaces the rows whole —
  a run that dies mid-harvest leaves the previous rows intact. The
  restore reads the rows in order and appends each to the file its
  tag names, through the same ingest that used to take them off a
  socket: the database is the source of the frames, and nothing
  else changed. Large databases and large memories are the normal
  case eventually.
- Restored ONCE, on the program's first run, and never again: the
  files stay on disk for the program's life, Hermes appends to them
  itself, and the database on disk is the only authority on the
  session. A later run reads no row and writes no continuation file;
  it asks `state.db` for the lineage's tip. Every run still harvests
  at its end. A restore that failed partway is retried by the next
  run, the three files cleared first.
- Harvest AFTER the gateway process has exited, and fold the
  database yourself: Hermes's close runs only a PASSIVE checkpoint,
  so a write-ahead log survives a clean exit and a database
  separated from it loses committed transactions. The module runs
  `VACUUM`, then `wal_checkpoint(TRUNCATE)`, then closes — and
  refuses a blocked checkpoint or a surviving log.
- Prove a restored `state.db` opens (`quick_check`) before the
  gateway starts: Hermes heals a database it cannot open by
  quarantining it and starting fresh, which would harvest an
  amnesiac continuation over the lineage.
- The restore answers the SESSION ID to resume, read from the
  database on that same connection — the most recently active row
  of `sessions` (`last_activity_at`, else `started_at`). The
  database is the only authority: compaction splits a session into
  a child row, so the id a run started with is not necessarily the
  lineage's tip when it ends. No rows, or no session yet, is the
  fresh start.
- Never enable session retention pruning in the config the harness
  writes; it would delete the older part of a lineage from inside
  the continuation.
- The MCP schema cache under `cache/` must never travel: a stale
  cache shows the model tools that no longer exist.
- Mounts land at the same paths on every run of a lineage: session
  rows record `cwd` and the git root.
- The `memory` toolset is in the vocabulary: its two files are the
  continuation's, so what it writes is what the next run starts
  with. External memory-provider plugins stay off.

## Nothing rides the container surface

There are no resources. What used to be fetched by identity over
the socket — rotating OAuth state — comes from the vault (above),
and the continuation comes from the database. Nothing rides the
container surface but the request and the chunks.

## The runner (`run::run`)

One stream for the whole run: plan → the vault cycle's first half →
prepare → the session (restored once, else the tip on disk) → spawn
`hermes gateway` (env from `prepare`, SIGTERM to stop, `/health`
polled every 250ms with no timeout) → turns over `/v1/runs` → stop →
read back and set the documents, unlock → harvest into the rows.
Rules settled with it:

- The queue is taken at each turn's end, and only there (above).
- Tool chunks come from the gateway's events, which carry no ids
  and no results: a FIFO of open calls pairs each `tool.completed`
  with the oldest `tool.started`, and the pair's id is minted here
  (random, base62, 22 chars). The call's `arguments` is the started
  event's preview; the response is EMPTY (`is_error` from the
  event) — for built-ins and proxied calls alike, until a byte-
  faithful source exists. A mismatched or unmatched completion
  flushes the FIFO and says so.
- `run.completed.output` is spoken as text only when no
  `message.delta` came (a provider that did not stream); otherwise
  it repeats the deltas and is dropped. Its usage is the bill.
- `approval.request` is answered `once` the instant it fires.
- After each turn the session tip is re-read from the database
  (compaction may have rotated it) and the next turn records into
  it. Effort rides `model_options.reasoning`.
- Before the gateway is up, a failure is the stream's one `Err` — the
  server's status; after, every failure is a fatal notification and
  the run still stops the gateway, releases the vault and harvests.
- One run at a time: the run holds the `Claim` (cc's three-phase
  lock) inside a `Teardown` captured into the stream; when the stream
  drops the teardown marks it settling, closes the queue on a task,
  and only then releases — a request landing meanwhile waits, never
  refused. The gateway dies with the stream; held locks lapse by TTL.

## The server

`main.rs` is the HTTP server around `run::run`, on the loopback at
the port the SDK's `container_proxy::agent` names (`PORT`, else
8080), forwarded to by the proxy the host injects: `POST /register` (the
agent, once, for the container's life; a second is `409`), `POST /run`
(the prompt JSON in — a run before registration is `409
unregistered`; the run's chunks out as server-sent events,
the first item pulled before the status is chosen — the run's one
`Err` is a `500` with its own words, a run with nothing to say is
`empty_run`, a run beside one streaming is `409 busy`), `GET
/schema` (`schemars::schema_for!(Agent)`), `POST /enqueue` (held
until the fate is known), `POST /dequeue`. Nothing of the proxy's is
touched before a request; the vault, the database and the gateway
are all `POST /run`'s.

## The runtime image (see also the Containerfile header)

`FROM nousresearch/hermes-agent`, pinned by tag AND digest to the
first release after the pinned source (v2026.8.31 for v0.20.6 /
4209d371); the build asserts `hermes version` is 0.20.6. The image
brings Python, the uv venv on PATH, aiohttp, playwright's chromium,
node, ripgrep, ffmpeg, git. We add `ddgs`, `edge-tts`, `fal-client`
into its venv (lazy installs are disabled there) and our one binary
as the entrypoint in place of its s6 dispatcher, so no gateway
auto-starts. The container proxy is NOT in the image: the host
injects it at runtime, and it may not be up until a request comes. The harness sets `HERMES_HOME=/root/.hermes` and
`HERMES_WRITE_SAFE_ROOT=` (empty — the image's `/opt/data` root
would deny writes to the caller's mounts) on the gateway process.
Never export `PYTEST_CURRENT_TEST` into the gateway environment
(auth.json access hard-errors under it).
