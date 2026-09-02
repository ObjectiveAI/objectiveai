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
- `model.api_key` in config.yaml for `custom`.

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
  Hermes, and its rotated form surfaced back to the caller (the
  emit-back mechanism is defined at the protocol level, not here).

## `prepare` renders all of it, once

The `prepare` module turns the request's agent into the gateway's
process environment (returned to the spawner — the request's own
`environment` is the server's business and is never repeated),
`config.yaml`, and the credential files, in one pass:

- `config.yaml` is written as JSON (JSON is YAML): `model.provider`
  (the SDK marker's id IS Hermes's) + `model.default`, `model.api_key`
  for `custom` only, `security.protected_instruction_files: false`,
  the backend pins asked for (`web.search_backend` /
  `web.extract_backend` by plugin name — `tavily`, `exa`, `parallel`,
  `keenable`, `brave-free`, `searxng`, `firecrawl`; `tts.provider:
  elevenlabs` when keyed; `image_gen.provider` / `video_gen.provider:
  fal`), and the MCP proxy entry `diverge` →
  `http://127.0.0.1:8081/mcp` with `trust: full`, elicitation AND
  sampling disabled.
- Toolset exposure is the explicit list `platform_toolsets.api_server`
  (the only deterministic form; there is no `disabled_toolsets`
  config key). Present → in, `false` → out, unsaid → Hermes's own
  API-server default: on for web, browser, terminal, file,
  code_execution, vision, image_gen, todo, memory, session_search;
  off for video, video_gen, x_search, tts, homeassistant, spotify.
  `skills` always in; delegation, cronjob, clarify, computer_use,
  discord, yuanbao, context_engine, stt never.
- The API server needs a usable bearer: `API_SERVER_KEY` is minted
  per run (64 hex), `API_SERVER_HOST=127.0.0.1`,
  `API_SERVER_PORT=8642`; `API_SERVER_ENABLED` is inert and unset.
- Resources (a provider's OAuth state, spotify's) are fetched ALL AT
  ONCE and parsed as JSON objects; only then is each file written,
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
  `GET /v1/runs/{id}/events`. The stream's full contract is
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
`continuation` module is the shape; its rules:

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

The run asks on its SSE stream (`fetch_resource` items), the server
delivers at `POST /resource/{identity}` (bytes verbatim, chunked) /
`…/complete` / `…/error`, the store settles per identity, and
`ResourceFetcher::fetch` hands the run the whole document as UTF-8
text — decoded over the ASSEMBLED bytes only, never per chunk.

## The runtime image (see also the Containerfile header)

python3 + `hermes-agent` at the pin, `ddgs` (the keyless web
fallback is presence-detected), the edge-tts/fal extras, node +
chromium for the browser toolset (verify Hermes's launcher passes
container-appropriate flags — sandbox and /dev/shm), and no
`~/.hermes/.env`. Never export `PYTEST_CURRENT_TEST` into the
gateway environment (auth.json access hard-errors under it).
