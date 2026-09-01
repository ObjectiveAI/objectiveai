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
  (`~/.qwen/oauth_creds.json`) plus the harness's own token-free
  `providers.qwen-oauth` marker;
- vertex's service-account JSON to a file + `VERTEX_CREDENTIALS_PATH`;
- `model.api_key` in config.yaml for `custom`.

Rules that make this work (reports 10, 11):

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
  `GET /v1/runs/{id}/events`. The stream's full contract is report
  14 and the `response` module: 12 event types discriminated by the
  payload's own `event` key; `: keepalive` and `: stream closed`
  are SSE COMMENTS; EOF must always terminate (a mid-stream failure
  closes the socket with no sentinel); the queue buffers from POST
  time but serves exactly ONE subscriber, destructively, with a
  300s unsubscribed TTL.
- Tool responses are NEVER on that stream (report 12) — the
  container emits its `tool_response` chunks from its own MCP
  proxy's view, the only byte-faithful one. The gateway events
  supply timing, reasoning glimpses, deltas and the bill.
- The enqueue/dequeue queue lives in the proxy; pending prompts
  fold onto the next tool response AT ITS TAIL (`\n\n` + one
  `<system-reminder>` section), and the fold's SHA-256 (lowercase
  hex over the folded text blocks, concatenated, no separators) is
  the caller's correlation key.

## Toolsets are applied, not passed through

The agent's toolset structures become env vars + config keys the
harness writes: enable/disable via Hermes's toolset config, backend
pins where a slot names one (`web.search_backend`,
`web.extract_backend`, `tts.provider: elevenlabs` when the key is
present). Deliberately out of the vocabulary and to be LEFT at
Hermes defaults or off: memory (the CONTINUATION is our cross-run
memory), context_engine, yuanbao, skills-as-a-switch (mounts decide
it), computer_use, stt, clarify, discord, cronjob, spotify's
managed-gateway sibling (the rotating Nous tool gateway stays
unsupported). Delegation needs NO provisioning: children are ad hoc,
inherit the parent's toolsets, credentials and our MCP proxy —
`delegation.subagent_auto_approve` is moot under yolo.

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
