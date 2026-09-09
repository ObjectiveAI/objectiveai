# The eliza harness — settled decisions

The operating rules for this container's implementation, accumulated
as they are settled. Each entry is a decision already made, with its
evidence in `reports/` where one exists (`reports/4.md` is the design
this implements; where the two differ, this file is the ruling). Add
to this file as more is figured out.

## The continuation is the database

Eliza's entire state — agents, memories, embeddings, entities, rooms,
worlds, relationships, logs, cache, tasks, every plugin's tables —
lives in the caller's database from the first turn, written by Eliza
and read by Eliza: `@elizaos/plugin-sql` speaks real Postgres, and
`POSTGRES_URL` is the container SDK's `postgres_url()`, the proxy's
loopback pgwire. There is nothing to restore before the runtime
starts and nothing to harvest after it stops. A run abandoned by its
caller leaves the database as the runtime left it: consistent up to
the last committed write.

What the harness itself remembers is one row of one table, in
`lineage.rs`:

```sql
CREATE TABLE IF NOT EXISTS eliza_lineage (
    id smallint PRIMARY KEY CHECK (id = 1),
    agent_id uuid NOT NULL,
    room text NOT NULL,
    embedding_dimensions integer,
    plugins jsonb NOT NULL
)
```

- `agent_id` is the identity: minted (v4) on the lineage's first run
  and pinned into the runtime constructor's `agentId` after. Every
  Eliza id derives from it (`createUniqueUuid(runtime, external)` is
  `stringToUuid("<external>:<agentId>")`); the character carries no
  `id`, so a renamed character keeps its memory.
- `room` is the conversation's external id, the constant `diverge`;
  the entity is `diverge:user`. Same agent id, same room, is the
  resumption; there is no session tip to compute.
- `embedding_dimensions` is what the vectors were built at; a run that
  names another width is told so in a non-fatal notification (Eliza
  re-embeds in the background, at the caller's cost).
- `plugins` records each caller plugin with the version the registry
  resolved; a later run installs `name@<that version>` whatever the
  agent's spec says.

Read at the top of every run and cached in memory after the first;
written BEFORE the runtime starts — on the first run, and whenever the
plugin set or the width changed — so a run that dies leaves a row
naming the agent id its memories carry. The row is read and written
on one sqlx connection beside Eliza's own `pg` pool; both go through
the proxy's pgwire.

## Settings go in the constructor map AND the process environment

`runtime.getSetting()` never reads `process.env` (multi-tenant rule,
`packages/core/CLAUDE.md`); its order is character secrets →
character settings → `settings.extra` → `settings.secrets` →
`character.env` → the constructor `settings` map. Some plugins add a
`process.env` tail of their own (plugin-openai, plugin-embeddings);
`plugin-sql` reads `POSTGRES_URL` through the core path with no shim.
So `settings.rs` renders EVERY setting and every secret into the
constructor map, and the same map into the entry process's
environment. Nothing goes on the character's `settings` or `secrets`,
so nothing lands in the `agents` row encrypted.

The map beats the row: `initialize()` merges the persisted `agents`
row's settings back into `character.settings`, but for every key the
constructor map names the map's value wins (`runtime.ts:3156-3175`).
That is how "the vault's copy at the top of Eliza's precedence" is
kept: a rewound row never wins over the vault for a key the harness
sets.

What the agent renders to:

- provider: `OPENAI_BASE_URL`, and the five tiers `OPENAI_NANO/SMALL/
  MEDIUM/LARGE/MEGA_MODEL` ALL set to `provider.model`;
  `OPENAI_API_KEY` from the vault. plugin-openai's `TEXT_EMBEDDING`
  handler is deleted from the plugin object by the entry — the agent's
  `embedding` is the one source of vectors — and its `init` (which
  registers the image/description/transcription/TTS tiers) is dropped
  unless `toolsets.generate_media` is on.
- embedding (present): plugin-embeddings loaded; `EMBEDDING_BASE_URL`,
  `EMBEDDING_MODEL`, `EMBEDDING_DIMENSIONS`; `EMBEDDING_API_KEY` from
  the vault. Absent: no embedding plugin, no `TEXT_EMBEDDING` handler
  at all — Eliza boots with a warning, stores memories without
  vectors and searches by keyword.
- toolsets: `coding_tools` → plugin-coding-tools with
  `CODING_TOOLS_WORKSPACE_ROOTS=/` and `SHELL_ALLOWED_DIRECTORY=/` (the
  container is the sandbox); `browser` → plugin-browser; `documents`
  → plugin-documents, constructor `enableDocuments`, `DOCUMENTS_PATH=
  /documents`, `LOAD_DOCS_ON_STARTUP=true` (a caller mounts the files
  there); `web_search` → plugin-web-search with `TAVILY_API_KEY` from
  the vault; `generate_media` → the `GENERATE_MEDIA` action, a
  basic-capabilities action always registered, is UNREGISTERED after
  `initialize()` when the switch is off. plugin-openai's media tiers
  (image description and generation, transcription, speech) are
  registered regardless — they are how a tool's image or audio is
  read to the model — and `OPENAI_IMAGE_DESCRIPTION_MODEL` is the
  caller's one model like the five text tiers; transcription keeps
  the plugin's default.
- memory: `advanced_capabilities` and `relationships` are constructor
  options; `advanced_memory` and `advanced_planning` are character
  flags.
- each caller plugin's `settings`: strings as-is, booleans and numbers
  spelled out, objects and arrays as JSON text, null not set.
- the vault's values are inserted LAST, so a plugin setting cannot
  shadow a secret of the same name.

Process-only, in the environment and not the map: `ELIZA_STATE_DIR=/
var/lib/eliza`, `ELIZA_VAULT_PASSPHRASE`, `ELIZA_VAULT_DISABLE_KEYCHAIN
=1`, `SECRET_SALT`, `ELIZA_DISABLE_TRAJECTORY_LOGGING=1`. `NODE_ENV` is
never set (production without a salt throws; unset is dev defaults).

## Secrets come from the vault, and only from the vault

A secret is a vault key named by the setting it fills. `vault.rs`
reads the keys off the agent: implied (`OPENAI_API_KEY` always,
`EMBEDDING_API_KEY` with an embedding, `TAVILY_API_KEY` with web
search) and every plugin's `secrets`. A key the vault does not hold,
or holds as something other than text, refuses the run before the
runtime starts, naming the key.

- STATIC (a bare name, or `rotates: false`): one `get`, no lock, no
  set.
- MINTED, the harness's own: `ELIZA_VAULT_PASSPHRASE` and
  `SECRET_SALT`. Read from the vault; absent, minted (two v4 uuids, 64
  hex characters) and set, so a lineage never changes them and never
  has to provision them. The salt is NOT a throwaway (report 3 said it
  was): `initialize()` merges the persisted row back into the
  character, and a settings value encrypted under another salt throws
  `Failed to decrypt secret setting` — a plugin that persisted one
  through `updateAgent` would kill every later run.
- ROTATING (`{key, rotates}`): Hermes's cycle. `vault_lock` for 300 s,
  re-locked every 100 s on a task for the run's life; `vault_get`;
  the value as the setting; after EVERY turn (and after a turn that
  ended in failure) the rotated value is read back from where
  `rotates` says and `vault_set` the moment it differs from the last
  value set or read — a container that dies mid-run has already put
  the last rotation where the next run finds it; `vault_unlock` at
  the end, every key attempted, the first failure reported. A run
  abandoned stops refreshing and the locks lapse by TTL.
- The read-back channels: `true` → `runtime.getSetting(key)` through
  the entry (`read setting`); `{setting}` → the same for that name;
  `{file: [...]}` → the file's bytes, read by the harness from the
  container's root; `{vault: entry}` → Eliza's own vault through the
  entry (`read vault`), `{agent_id}` in the entry name replaced with
  the lineage's. A read-back that finds nothing is nothing new; a key
  nothing was EVER read back for is a non-fatal notification at the
  run's end, naming it.
- Never the database: the caller's database is snapshotted and
  rewound, and a credential rewound is a login burned. A plugin that
  keeps a rotating credential in a table of its own is unsupported,
  on purpose; Eliza's cache is the database too and is not a channel.

## Eliza's own vault, on by default

`@elizaos/vault` is a library plugins import directly (browser,
telegram, wallet, google-workspace, even plugin-sql); the core has no
hook for it. The harness gives it what a headless host needs every
run: the passphrase in `ELIZA_VAULT_PASSPHRASE` (the minted static
above; the library wants twelve characters or more), the keychain
probe off, and the store under `ELIZA_STATE_DIR=/var/lib/eliza` at
`.vault-pglite/`. Fixed always; a caller who wants the store to
outlive the container mounts a volume there, and one who does not
gets a store that dies with it. The entry reads an entry back with
`createVault().get(key)` — the same default construction the plugins
use, so the master key and the data directory agree.

## The entry, and its line protocol

The harness never runs the `@elizaos/agent` host. `node/entry.mjs` is
the harness's own driver over `@elizaos/core`: one process per run
(the agent's plugin set may differ between runs), spawned by
`run/entry.rs` with `cwd=/opt/diverge/eliza`, its stdin and stdout the
harness's, its stderr Eliza's logger passed through and never parsed.
One JSON object per line, discriminated by `type`; the Rust half is
`run/protocol/`, and there is no third copy.

Harness → entry: `configure` once, first (`agentId`, `character`,
`settings`, `plugins` — the image's, in order — `installed` — the
caller's — `openaiMedia`, `advancedCapabilities`,
`enableRelationships`, `enableDocuments`, `mcpUrl`); `turn {text}`;
`read {kind: setting|vault, key}`; `stop`.

Entry → harness: `fatal {error}` before ready (exit 1); `ready` after
`initialize()`, `ensureConnection` and the `MODEL_USED` subscription;
per turn `text {delta}`, `tool_call {id, name, arguments}`,
`tool_result {id, result}`, `usage {prompt, completion, total}`,
`notification {message}`, then `done {text, streamed, failure}`;
`value {key, value}` answering a read; `stopped` after
`runtime.stop()` and `runtime.close()` (exit 0).

Rules settled with it:

- `runtime.stop()` runs in its ORDINARY mode, never fast: the
  post-turn work — facts, reflection, the embedding queue — is the
  lineage's memory, and it lands only if the runtime finishes it. The
  harness awaits `stopped` and the exit however long they take.
- The runtime is constructed with `checkShouldRespond: false` (a
  direct chat interface always answers), `logLevel: "warn"`, and the
  export of each package found the way the host finds it (`default`
  → `plugin` → any `*Plugin`-named export that has a name, a
  description and one capability array or an `init`).
- Identity: `entityId = createUniqueUuid(runtime, "diverge:user")`,
  `roomId = createUniqueUuid(runtime, "diverge")`, one
  `ensureConnection({..., type: ChannelType.DM})` before ready —
  `memories.room_id` has a foreign key, so the room must exist. The
  inbound memory is `createMessageMemory` without `agentId` (scope
  `shared`).
- A turn is one `handleMessage(runtime, memory, callback, {abortSignal,
  onStreamChunk})`. The turn's result is checked on both the resolved
  and the thrown path; `result.terminalFailure` (or the throw) is the
  `done`'s `failure`.
- Abandoned mid-run, the entry dies with the stream (`kill_on_drop`);
  an entry that stopped answering is killed and reaped.

## The stream envelope, and the chunk vocabulary

`onStreamChunk(chunk, messageId, accumulated, streamRevision)` carries
text deltas AND JSON strings discriminated by `type`: `tool_call`,
`tool_result`, `evaluation` (their payload spread flat) and
`context_event` (payload under `event`). When `accumulated` is a
string it is the truth and may restart — the delta is what it grew
by, else the chunk (the shipped `agent-client.ts` logic, verbatim).
`result.responseContent.text` is authoritative; the stream is
presentation. Mapped:

- Text deltas → `assistant_text_content`. The final text is spoken
  only when NO delta came (a provider that did not stream); otherwise
  it repeats the deltas and is dropped — hermes's rule.
- The diverge plugin's calls → `assistant_tool_call` (id minted by the
  plugin, the tool's MCP name, the arguments as JSON text) and
  `tool_response` with the MCP `CallToolResult` verbatim, byte-
  faithful. The runtime's own `tool_call`/`tool_result` envelopes for
  `DIVERGE_*` actions are dropped, so each call is said once; so are
  `REPLY`, `IGNORE` and `NONE` (the reply IS the text). Every other
  action's envelopes convert with Eliza's own `toolCall.id` (minted
  when it has none) and its result rendered as one text block,
  `isError` from `status === "failed"`.
- `evaluation` and `context_event` → non-fatal notifications, in
  Eliza's own words.
- Usage: `MessageProcessingResult` carries none. The one channel is
  `EventType.MODEL_USED`, emitted per model call by the provider
  plugin; each event is one `usage` chunk (a delta, as the chunk is).
  A call without token counts, and a turn without any usage event,
  are non-fatal notifications rather than an invented bill.
- A delivered queue message → `user`.
- `done.failure` → a fatal notification, and the run ends (the
  read-back and the stop still run).

## The queue is consulted when a turn ends

Nothing steers a `handleMessage` mid-way (the `abortSignal` is for
the teardown, not for steering: a turn cut short would leave half a
response in the lineage's memory). The container holds its own queue
(`queue.rs`, openrouter's as hermes carries it), and takes it at each
turn's end: every pending message answered `delivered`, yielded as a
`user` chunk, joined with a blank line between into the next turn's
input in the same room. An empty queue at a turn's end closes it in
the same lock hold and ends the run; later enqueues are `missed`.
Invoking Eliza again IS the delivery.

## The diverge plugin is the tool channel, and the resource channel

`node/diverge.mjs`: before the runtime is constructed, inside `POST
/run`, an MCP `Client` over `StreamableHTTPClientTransport` at the
container SDK's `mcp_url()` lists every tool and every resource
(paginated) and builds one native `Action` per tool: name `DIVERGE_`
+ the tool name upper-cased with every character outside `[A-Z0-9]`
as `_` (the runtime's pattern; a collision gets a numeric suffix),
the tool's description, one `ActionParameter {name, description,
required, schema}` per `inputSchema.properties` entry (the runtime
folds these back into one tool schema for native tool calling),
`validate` always true, and a handler that calls `callTool` and
returns `{success: !isError, text: <text blocks joined>, data: {mcp:
result}}` — `text` is what the model reads back, with no extra model
call. plugin-mcp was rejected: it paraphrases every result through a
`TEXT_SMALL` call, exposes one untyped `MCP` action, and its SSRF
guard may refuse the loopback.

Non-text content reaches the model the way an inbound attachment
does. The planner renders an `ActionResult` into the model's tool
result by JSON-stringifying the WHOLE object as text — there is no
image or file part on that path at the pin, and `ActionResult` has no
attachments — so the plugin's `ActionResult` is `{success, text}` and
nothing else (never the result in `data`: a base64 block there would
land in the model's context as a giant string), and `text` is each
block rendered as Eliza's own ingress renders an attachment:

| block | the model's text |
|---|---|
| `text` | the text |
| `image` | `describeImageCached` (core's cached `IMAGE_DESCRIPTION`) over the data URL, with core's own prompt → title and description; no model registered → `[image <mime>, <n> bytes; …]` |
| `audio` | `useModel(TRANSCRIPTION, bytes)` → the transcript; no model → `[audio …]` |
| embedded `resource` / read `contents`, text | `Resource <uri> (<mime>):` then the text |
| embedded `resource` / read `contents`, blob | by mime: `image/*` as image; `audio/*` as audio; `text/*` and `application/json` as UTF-8; `application/pdf` through `unpdf`'s `extractText` (core's own PDF call; the parser itself is not exported); else `[binary <uri>, <mime>, <n> bytes]` |
| `resource_link` | `Resource link <uri> — <name> (<mime>); readable with DIVERGE_READ_RESOURCE` |

A transcoder that throws yields `[… could not be read: <error>]` in
place, and the run goes on. The `tool_result` line to the harness —
and so the `tool_response` chunk — stays the MCP result verbatim.

The lists are LIVE. The proxy's MCP server declares `tools/
list_changed` and `resources/list_changed` and broadcasts the
caller's notifications to every initialized peer; the plugin
subscribes to both with the SDK client's `setNotificationHandler`:

- A tool change re-lists and diffs by MCP tool name: a removed tool
  is `unregisterAction`ed, a new one registered, one whose
  description or input schema changed is unregistered and registered
  again under the SAME action name (names are stable for the run; a
  new tool takes a name avoiding the ones in use). The caller sees it
  as a non-fatal `notification` `{kind: "tools_changed", added,
  removed, changed}`.
- A resource change re-lists into the cache the `DIVERGE_RESOURCES`
  provider renders; `{kind: "resources_changed", added, removed}`.
- Refreshes run one at a time per list, in notification order; a
  re-list that fails keeps the previous set and says so (`{kind:
  "mcp", list, error}`). A notification that arrives before the
  runtime exists only marks the list dirty; the entry calls the
  plugin's `flush()` right after `initialize()` — with the static
  actions certainly registered — so a refresh never races the
  initial registration.
- A change lands on the NEXT model call, not the one in flight: the
  planner's tool schemas are built per call from the current actions.

Resources: the provider `DIVERGE_RESOURCES` (`alwaysInResponseState`,
so context routing never drops it) renders one line per resource —
`<uri> — <name>: <description> (<mimeType>)` — into every turn's
state, and nothing when there are none. The action
`DIVERGE_READ_RESOURCE` (its name reserved before any tool folds, so
a tool literally named `read_resource` becomes
`DIVERGE_READ_RESOURCE_2`) takes one required `uri`, calls
`readResource`, and is reported like a tool: a `tool_call` named
`read_resource` with `{"uri"}`, and a `tool_result` whose content is
the read's contents verbatim as MCP embedded-resource blocks
(`{type: "resource", resource: {uri, mimeType, text | blob}}`, which
rmcp's `CallToolResult` deserializes as-is). The model reads the text
contents joined; a blob is described (`binary <uri> (<type>, <n>
base64 characters)`), never pasted. Resource templates are not
listed: the proxy relays no `resources/templates/list` exchange.

## Plugins the caller names are installed at the run

`plugins.rs`: every package the agent's `plugins` names is installed
at `POST /run`, before the runtime starts, with `bun add
--ignore-scripts <spec>` in the project — the command elizaOS's own
installer runs — at the lineage's pinned version where it has one,
else the agent's spec. The resolved version is read from the
installed package's manifest and recorded in the row. Installs are
cached by spec for the program's life. The install is the one thing a
run does before the proxy is touched, and it is the registry's, not
the proxy's. A spec the registry cannot resolve, or a package that
exports no plugin once imported, refuses the run before the runtime
starts. An installed plugin is arbitrary JavaScript in the agent's
process with the caller's tools in reach; the container is the
sandbox.

## One run at a time, and the settlement releases the lock

cc's three-phase `Claim` (Idle / Running / Settling), inside a
`Teardown` captured into the run's stream: when the stream drops the
teardown marks the claim settling, closes the queue on a task, and
only then releases — a request landing meanwhile waits, never
refused. The entry dies with the stream; held vault locks lapse by
TTL.

## The server

`main.rs` is hermes's server around `run::run`, on the loopback at
the port the SDK's `container_proxy::agent` names (`PORT`, else 8080),
forwarded to by the proxy the host injects: `POST /register` (the
agent, once, for the container's life; a second is `409`), `POST /run`
(a run before registration is `409 unregistered`; a run beside one
streaming is `409 busy`; an empty prompt `400`; a database that will
not answer `500`; the run's one `Err` — lineage, vault, install, the
entry failing before ready — is a `500` in its own words; a run with
nothing to say is `empty_run`; then the chunks as server-sent events,
the first item pulled before the status is chosen), `GET /schema`
(`schemars::schema_for!(Agent)`), `POST /enqueue` (held until the fate
is known), `POST /dequeue`. Nothing of the proxy's is touched before a
request.

## The runtime image (see also the Containerfile header)

`node:24-bookworm-slim` by digest, `bun` copied from `oven/bun:1.3.14`
(the pin's `packageManager`) for the installs, `ca-certificates`,
`git`, `python3`. The Node project at `/opt/diverge/eliza`:
`@elizaos/core`, `plugin-sql`, `plugin-openai`, `plugin-embeddings`,
`plugin-coding-tools`, `plugin-browser`, `plugin-documents`,
`plugin-web-search` and `vault` pinned to `2.0.3-beta.7` — the version
of the source under `sources/`, and, checked on 2026-09-09, published
at exactly that version — plus `@modelcontextprotocol/sdk` at the
pin's own newest spec. `bun install` at build with scripts allowed
(native dependencies build then, never at a run); no lockfile is
committed. The harness binary is the entrypoint. The container proxy
is NOT in the image.

## What a live run has to prove

Nothing here has been built or run. In order of what would break
first:

1. `bun install` of the nine pinned packages into the project, and
   the entry importing each (`plugin-sql` exports `plugin`,
   `plugin-browser` exports `browserPlugin`, the rest `default`).
2. The proxy's pgwire accepting Eliza's `pg` pool beside the harness's
   sqlx connection, `CREATE EXTENSION IF NOT EXISTS vector`
   (`fuzzystrmatch`, `pg_trgm`) against the caller's database, and the
   startup migration's cost on a cold lineage.
3. The stream envelope in practice: that `DIVERGE_*` envelopes are
   the ones dropped and the diverge plugin's own lines are the ones
   kept, and that `accumulated` behaves as the shipped client assumes.
4. `MODEL_USED` firing per call through plugin-openai against the
   caller's endpoint, with token counts.
5. `bun add --ignore-scripts` of an arbitrary `@elizaos/plugin-*` at a
   run, and its plugin loading through the constructor.
6. `@elizaos/vault`'s single-writer PGlite when the entry's
   `createVault()` opens the store beside a plugin's own instance.
7. The embedding width probe with the caller's endpoint, and the
   warning when a lineage changes width.
8. Node's startup plus `initialize()` per run, to decide whether the
   entry should instead be kept alive across runs whose agent value
   hashes the same.
9. The SDK client's standalone GET stream through the proxy's
   `StreamableHttpService`: that a `list_changed` the caller sends
   reaches the plugin's notification handler at all.
10. A tool re-registered mid-run showing up on the planner's next
    call, and a removed one gone, with `registerAction` /
    `unregisterAction` on the live runtime.
11. An image block described through the caller's endpoint with the
    caller's model, an audio block transcribed with the plugin's
    default transcription model, and a PDF blob read through `unpdf`.
