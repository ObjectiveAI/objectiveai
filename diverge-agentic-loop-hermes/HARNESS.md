# The hermes harness — settled decisions

The operating rules for this container's implementation, accumulated
as they are settled. Each entry is a decision already made, with its
evidence in `reports/` where one exists. Add to this file as more is
figured out.

## Auth is handled on the filesystem, from the request

Nothing about credentials arrives ambiently: the request's agent
carries them as arguments (provider structures, toolset structures,
`*_resource` identities), and the harness APPLIES them before
`hermes gateway` starts —

- canonical env vars in the gateway's PROCESS environment (the
  per-provider and per-toolset docs in the SDK name each one);
- `$HERMES_HOME/auth.json` entries for the resource providers
  (nous, openai-codex, minimax-oauth) and spotify — the fetched
  resource bytes written verbatim as `providers.<name>`;
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
- Rotating OAuth state is a RESOURCE: fetched by identity over the
  container surface, written to the filesystem, rotated in place by
  Hermes, and its rotated form surfaced back to the caller as a
  `resource` frame — tag `3` on the container surface, `2` on the
  wire: a u32 name length, the name (the request field's dotted
  path, `provider.auth_resource` / `toolsets.spotify.auth_resource`),
  then the whole new document. Not terminal; the last one wins.

## The `filesystem` module lays all of it down, once — and streams it back

Two entry points: `filesystem::prepare` before the gateway,
`filesystem::finish` after it has exited. `finish` streams every
resource the request named, as the run left it (the `auth.json`
entry re-serialized, the Qwen file verbatim — a resource no longer
there yields nothing, since Hermes quarantines a terminally-failed
entry and the caller keeping its copy beats a lost run), and THEN
the continuation, the closer, a piece at a time.

`filesystem::prepare` turns the request's agent into the gateway's
process environment (returned to the spawner — the request's own
`environment` is the server's business and is never repeated),
`config.yaml`, and the credential files, in one pass — and awaits
the continuation's settlement (its chunks having landed on disk as
they came, `filesystem::continuation`) IN PARALLEL with the resource
fetches, since the two halves write disjoint files:

- `config.yaml` is written as JSON (JSON is YAML): `model.provider`
  (the SDK marker's id IS Hermes's) + `model.default`,
  `model.base_url` + `model.api_key` for `custom` only, `security.protected_instruction_files: false`,
  the backend pins asked for (`web.search_backend` /
  `web.extract_backend` by plugin name — `tavily`, `exa`, `parallel`,
  `keenable`, `brave-free`, `searxng`, `firecrawl`; `tts.provider:
  elevenlabs` when keyed; `image_gen.provider` / `video_gen.provider:
  fal`), and the MCP proxy entry `diverge` →
  `http://127.0.0.1:8081/mcp` with `trust: full`, elicitation AND
  sampling disabled.
- Toolset exposure is the explicit list `platform_toolsets.api_server`
  (the only deterministic form; there is no `disabled_toolsets`
  config key). `true` or a structure present → in; `false` or a
  structure absent → out. Every switch is stated (the plain ones are
  required bools, the argument-taking ones `Option` structures) and
  nothing is on by omission: Hermes's own API-server default — web,
  browser, terminal, file, code_execution, vision, todo, memory,
  session_search, and image_gen (hidden without a FAL key, which only
  the structure brings) — is never consulted.
  `skills` in exactly when something is mounted under the external
  skills path, out otherwise — and skills come from MOUNTS at
  `/root/.hermes/external-skills/` (one directory per skill, its
  `SKILL.md` inside), which the config names as `skills.external_dirs`:
  Hermes discovers them recursively and views them, never writes
  (read-only to the curator, the usage tracker, the hub). Its own
  `skills/` is off limits for mounts — bundled skills sync into it
  at startup and the bookkeeping lives there. delegation, cronjob,
  clarify, computer_use,
  discord, yuanbao, context_engine, stt never.
- `HERMES_HOME=/root/.hermes` is set explicitly, pinning the geometry
  the files were laid down under.
- The API server needs a usable bearer: `API_SERVER_KEY` is minted
  per run (64 hex), `API_SERVER_HOST=127.0.0.1`,
  `API_SERVER_PORT=8642`; `API_SERVER_ENABLED` is inert and unset.
- Resources (a provider's OAuth state, spotify's) are fetched ALL AT
  ONCE and parsed as JSON objects, the continuation fetched beside
  them (every ask reaches the socket through the driver, on the one
  `Fetcher`'s channel); only then is each file written,
  exactly once — `auth.json` as `{"version": 1, "providers": {...}}`
  with the documents verbatim, the Qwen file, the vertex file
  (`VERTEX_CREDENTIALS_PATH` names it).
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
- The enqueue/dequeue queue lives in the proxy; pending prompts
  fold onto the next tool response AT ITS HEAD (one
  `<system-reminder>` section, then a blank line, then the tool's
  own content), and the fold's SHA-256 (lowercase
  hex over the folded text blocks, concatenated, no separators) is
  the caller's correlation key.

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
resources, caches regenerate, skill writing is unsupported. The
`filesystem::continuation` module is the shape; its rules:

- The continuation is NEVER held whole in memory: a delivered chunk
  is appended to the file its tag names the moment it lands (the
  slot keeps only the open handle), and the harvest streams each
  file out in pieces of at most 2 MiB, one alive at a time. Large
  databases and large memories are the normal case eventually.
- Harvest AFTER the gateway process has exited, and fold the
  database yourself: Hermes's close runs only a PASSIVE checkpoint,
  so a write-ahead log survives a clean exit and a database
  separated from it loses committed transactions. The module runs
  `VACUUM`, then `wal_checkpoint(TRUNCATE)`, then closes — and
  refuses a blocked checkpoint or a surviving log.
- Prove a delivered `state.db` opens (`quick_check`) before the
  gateway starts: Hermes heals a database it cannot open by
  quarantining it and starting fresh, which would harvest an
  amnesiac continuation over the lineage.
- The continuation fetch answers the SESSION ID to resume, read
  from the database on that same connection — the most recently
  active row of `sessions` (`last_activity_at`, else `started_at`).
  The database is the only authority: compaction splits a session
  into a child row, so the id a run started with is not necessarily
  the lineage's tip when it ends. `None` is the fresh start.
- Never enable session retention pruning in the config the harness
  writes; it would delete the older part of a lineage from inside
  the continuation.
- The MCP schema cache under `cache/` must never travel: a stale
  cache shows the model tools that no longer exist.
- Mounts land at the same paths on every run of a lineage: session
  rows record `cwd` and the git root.
- On the wire the protocol KEEPS chunk boundaries (nobody joins or
  splits a continuation's pieces), so each chunk leads with one tag
  byte naming its file — `0` state.db, `1` MEMORY.md, `2` USER.md.
  The ingest appends each chunk to its tag's file and judges nothing
  else: the container is fresh and the delivery lands before the
  gateway starts, so there is nothing stale and no order to police.
- The `memory` toolset is in the vocabulary: its two files are the
  continuation's, so what it writes is what the next run starts
  with. External memory-provider plugins stay off.

## Resources ride the container surface

The run asks on its socket (`fetch_resource` frames), the server
delivers at `POST /resource/{identity}` (bytes verbatim, chunked) /
`…/complete` / `…/error`, the store settles per identity, and
`Fetcher::fetch_resource` hands the run the whole document as UTF-8
text — decoded over the ASSEMBLED bytes only, never per chunk. The
continuation is fetched the same way (`Fetcher::fetch_continuation`),
by the same object, on the same ask channel; only the stores differ
(`store::resource` in memory by identity, `store::continuation` one
slot to disk).

## The runner (`run::run`)

One stream for the whole lifetime: prepare → spawn `hermes gateway`
(env from `prepare`, SIGTERM to stop, `/health` polled every 250ms
with no timeout) → turns over `/v1/runs` → stop → `finish`. Rules
settled with it:

- The proxy tells us nothing but the RETURN of its `/enqueue`,
  which happens at the fold. That return's moment is recorded and
  the prompt is yielded as a `user` chunk after the tool response
  whose `tool.completed` timestamp is the first at or after it
  (proxied calls are sequential barriers, so it is the one). The
  container keeps its own queue mirrored onto the proxy's; a fate
  is decided by whoever takes it first — the fold (`delivered`), the
  caller's dequeue (`dequeued`), or the turn's end (`delivered`: the
  message opens the next turn as its prompt, the proxy told to fold
  nothing stale first). An empty queue at a turn's end closes the
  run; later enqueues are `missed`.
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
- Before the gateway is up, a failure is the stream's one `Err`;
  after, every failure is a fatal notification and the run still
  stops the gateway and closes with the continuation.

## The socket

`main.rs` is the socket around `run::run`: the request frame read,
one `Fetcher` built, the run started, and then ONE merged stream
onto the socket — the fetcher's asks (`fetch_resource`,
`fetch_continuation`; the server's to consume) and the run's items
(chunks, rewritten resources, the continuation's pieces last), each
as its frame, in the order they come. The ask channel ends when the
fetcher is dropped after the filesystem is prepared; the items end
when the run does. A failure before the gateway is up is the run's
one `Err` and becomes a fatal notification; after, the run's own
fatal notifications ride the stream and the closer still comes.

## The runtime image (see also the Containerfile header)

`FROM nousresearch/hermes-agent`, pinned by tag AND digest to the
first release after the pinned source (v2026.8.31 for v0.20.6 /
4209d371); the build asserts `hermes version` is 0.20.6. The image
brings Python, the uv venv on PATH, aiohttp, playwright's chromium,
node, ripgrep, ffmpeg, git. We add `ddgs`, `edge-tts`, `fal-client`
into its venv (lazy installs are disabled there), our two binaries,
and our entrypoint in place of its s6 dispatcher, so no gateway
auto-starts. The harness sets `HERMES_HOME=/root/.hermes` and
`HERMES_WRITE_SAFE_ROOT=` (empty — the image's `/opt/data` root
would deny writes to the caller's mounts) on the gateway process.
Never export `PYTEST_CURRENT_TEST` into the gateway environment
(auth.json access hard-errors under it).
