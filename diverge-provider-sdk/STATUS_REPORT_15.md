# Report 15: registration, FUSE mounts, and three further containers

## 1. Scope and method

This report records the state of the provider protocol and its
containers as of commit `47b195df2` (2026-09-09), stated as a
differential against the state report 14 recorded at commit
`a8a982245` (2026-09-08). It was prepared from the complete
`git diff a8a982245..47b195df2` and from the files as they stand at
the later commit. It reports final positions only. Intermediate
designs that were adopted and then superseded within the range are
not described, except where a settled position is explained by the
alternative it replaced.

The range comprises forty-two commits and touches 255 files
(13,113 insertions, 837 deletions), distributed as follows:

| area | files | insertions | deletions |
|---|---|---|---|
| `diverge-provider-sdk` | 129 | 1,597 | 357 |
| `diverge-container-proxy` | 18 | 708 | 70 |
| `diverge-agentic-loop-cc` | 2 | 74 | 17 |
| `diverge-agentic-loop-hermes` | 3 | 76 | 21 |
| `diverge-agentic-loop-openrouter` | 2 | 73 | 19 |
| `diverge-agentic-loop-eliza` | 28 | 4,850 | 134 |
| `diverge-agentic-loop-codex` (new) | 43 | 3,468 | 0 |
| `diverge-agentic-loop-python` (new) | 24 | 2,184 | 0 |
| `diverge-agentic-loop-pi` (deleted) | 3 | 0 | 216 |
| `Cargo.toml`, `Cargo.lock`, `version.sh` | 3 | 83 | 3 |

`diverge-container-proxy-sdk` is unchanged in the range.

Verification status, stated once and applicable throughout: every
crate named here passes `cargo check` and `cargo doc --no-deps` with
zero warnings, the provider SDK under each of its feature
combinations. No container image has been built. Nothing has been run
against a live proxy, database, vault, or MCP server. The one
execution performed in the range is of the Python container's harness
script against a host CPython 3.13 with fixture inputs (section 9.4).

## 2. The agent is registered once; a loop carries its prompt

### 2.1 The wire

Report 14 placed both the prompt and the agent on the request that
creates an agent container, and made the `RunLoop` channel bare. That
position is withdrawn. The agent is now fixed for the container's
life and the prompt is per loop.

- `endpoints::containers::agents::run::client::request::Frame` is
  `{ container (flattened), agent: Value }`. The `prompt` field is
  removed.
- `endpoints::containers::agents::run::client::channel_request::Frame::RunLoop`
  (tag `5`) carries `shared::containers::run_loop::request::Request
  { prompt: String }`, encoded as its JSON. A new
  `FrameError::RunLoop(serde_json::Error)` reports a prompt that does
  not parse. `shared::containers::run_loop` gains the `request` module
  accordingly.
- The `run_loop` module's documentation now states that a container
  runs loops one after another, each resuming the conversation the
  previous one left, one at a time; a second opening while one runs
  is refused by the image as an `Error` frame.
- The typed reference enum `endpoints::containers::agents::agent::Agent`
  loses its `Eliza` variant and the `eliza` reference module is
  deleted (moved into the eliza crate; section 7). The reference enum
  is now `Codex | Python`. The `codex` and `python` reference modules
  remain in the SDK although both images now own their own agent
  types (sections 8 and 9); the module's own documentation says an
  image brought up to the containers API leaves this module, and for
  these two it has not yet done so.

### 2.2 The proxy path `/agent/register`

`container_proxy::agent::register` is a new server-opened path. The
server sends one message, `request::Request { agent: Value }`, and
the container answers one `response::Frame`: `Registered` (tag `0`)
or `Error(shared::error::Error)` (tag `1`), then the close. The
executor, behind the `server` feature, is
`container_proxy::agent::register::execute::execute(client, request)
-> Result<(), ExecuteError>`, with `ExecuteError::{Open, Encode,
Refused(Error), Frame(FrameError), Unserved, Text, Socket, Closed}`.
`Refused` carries the container's own reason.

The agent's server must refuse a `/run` before registration
(`{"kind":"unregistered"}`, `409`) and any second `/register`
whatever it carries (`{"kind":"registered"}`, `409`); both are
non-`2xx` and are forwarded as the path's `Error`. The proxy accepts
`/agent/register` as many times as the server opens it and forwards
each; the once-only rule is enforced by the agent's server.

`container_proxy::agent::run::request` now re-exports
`shared::containers::run_loop::request` (the prompt alone).

### 2.3 The proxy crate

`diverge-container-proxy/src/agent/register.rs` serves the path: the
first message must be a binary message and is forwarded verbatim as
the body of `POST /register` with `Content-Type: application/json`;
any other first message, or none, is the clean close with nothing
before it; a `2xx` is `Registered`; a non-`2xx` or a server that
cannot be dialed is `Error`, built by the same helpers the other
`/agent/*` paths use.

### 2.4 The containers

`diverge-agentic-loop-cc`, `-hermes` and `-openrouter` each gain
`src/registration.rs` (a `OnceLock<Agent>`; `register` returns the
agent back on a second call; `registered()` clones) and a
`POST /register` route: `400 {"kind":"agent"}` for a value the image
will not take, `409 {"kind":"registered"}` for a second registration,
`204` on success. `POST /run` now takes
`container_proxy::agent::run::request::Request` (the prompt) and
refuses `409 {"kind":"unregistered"}` before the claim is taken. The
parsing of the agent value at `/run` is removed from all three. The
eliza, codex and python containers were built with this shape from
the outset.

## 3. FUSE mounts

### 3.1 On the container request

`shared::containers::request::Container` gains
`fuse_mounts: Vec<FuseMount>` (default empty, omitted when empty).
`FuseMount { container_path: Vec<String>, id: String, readonly: bool
(default false) }`. The documented constraints: the path names a
file, never a directory or the root; no component is empty, `.` or
`..`; a path inside a directory another mount provides is permitted;
a path equal to another mount's is not; two mounts on one request may
not share an id. The provider must mount every one before the
container starts. The file's bytes are the caller's: fetched on every
open and stored on every changed close, each exchange carrying the
id. The stated purpose is credential files that vendor CLIs rewrite
when they refresh a login.

### 3.2 The shared vocabulary `shared::containers::fuse`

Two operations, binary throughout:

| operation | ask payload | answer (one message) |
|---|---|---|
| `read` | `[id: utf8…]` (the id is the whole payload) | `Frame`: `0` `Present(&[u8])`, `1` `Missing`, `2` `Error(&str)` |
| `write` | `[id_len: u16 BE][id…][bytes…]` | `Frame`: `0` `Ok`, `1` `Error(&str)` |

Error types: `RequestError::{Truncated, IdUtf8}`,
`RequestEncodeError::IdLength(usize)` (an id longer than 65,535
bytes; only `write` prefixes its id), `ResponseError::{Empty,
UnknownKind(u8), MessageUtf8}`. Rules stated in the module: the id is
opaque and the caller's; a file is one message in each direction,
neither paged nor chunked, so the mechanism is for files of
credential or configuration size; a read-only mount never produces a
`write`, and a caller that receives one anyway may answer `Error`; no
operation is retried.

### 3.3 The provider's channels toward the caller

Both `agents::run` and `tools::run` server-opened
`channel_request::Frame`s gain `FuseRead(fuse::read::request::Request)`
(tag `18`) and `FuseWrite(fuse::write::request::Request)` (tag `19`).
Twenty tags in each family, in the same order. `FrameError::Fuse` and
`FrameEncodeError::Fuse` are added. The client's answers are
`client::channel_response::{fuse_read, fuse_write}::Frame`, each an
alias of the shared response frame. The `connect` scope is unchanged
(`WriteBytes` only).

### 3.4 The proxy wire

`container_proxy::requests::request::Request` gains `FuseRead` (kind
`12`, `[id…]`) and `FuseWrite` (kind `13`, `[id_len: u16][id…]
[bytes…]`), answered on `/fuse/read/{channel}` and
`/fuse/write/{channel}` with one message then the close.
`container_proxy::fuse::{read, write}` re-export the shared
vocabulary and carry the `execute(client, channel, Option<&Frame>)`
executors under the `server` feature, `None` being the refusal (the
clean close with no message). `FrameError::Fuse` and
`RequestEncodeError::Fuse` are added to the requests frame.

### 3.5 `container_proxy::filesystem` and the mounts environment

`container_proxy::filesystem::MOUNTS_ENV` =
`DIVERGE_CONTAINER_PROXY_FILESYSTEM_MOUNTS`. Its value is
`Mounts(Vec<Mount>)` as a JSON array, `Mount { path: Vec<String>, id:
String, readonly: bool (default false) }`. A value that does not parse
is an error the proxy refuses to start over; an unset variable is no
mounts. This differs deliberately from the filetree ignore list, whose
unparseable value is treated as empty.

The documented semantics of a mounted file: one regular file, root's,
one link, size the caller's answer's length, mode `0600` (or `0400`
when read-only); `stat` asks the caller; a caller holding nothing
under the id answers an empty file; `open` snapshots into a
per-handle buffer; no lock; overwritable in place by every means
(`O_TRUNC`, `truncate` with or without a handle, `write` at any
offset including past the end); `flush`, `fsync` and the close of a
changed handle store the whole buffer, a refused store surfacing as
`EIO`; two write handles each store the whole buffer and the last
close wins; `chmod` and `chown` are accepted and change nothing;
read-only mounts carry the kernel `ro` option and additionally answer
`EROFS` on every write path; the mount point cannot be renamed or
unlinked (the kernel's `EBUSY`), so a program that saves by
temp-file-and-rename fails at the rename; the mount requires
`/dev/fuse` and the proxy running as root in the container's user
namespace; a mount that cannot be made ends the proxy at start.

### 3.6 The proxy crate's FUSE filesystem

`diverge-container-proxy/src/filesystem/mount.rs` implements the
above with `fuser 0.18.0` and `libc 0.2.186`, both under
`[target.'cfg(unix)'.dependencies]`; on a non-Unix host `mount()`
returns `io::ErrorKind::Unsupported`. `mount(requests, handle,
&Mount)` builds the path from `/` and the components, creates every
missing parent directory (`create_dir_all`), creates the file if
absent with the mode above without truncating an existing one, and
spawns a `fuser::Session` with options `FSName("diverge-fuse")`,
`DefaultPermissions`, `NoAtime`, and `RO` or `RW`; `SessionACL::All`;
one thread. Dropping the returned `Mounted` unmounts.

The filesystem has one inode (`ROOT`, a regular file). `lookup`
answers `ENOTDIR`; `getattr` and `setattr` on any other inode answer
`ENOENT`; attribute TTL is zero. `getattr` without a handle asks the
caller (`FuseRead`) and answers the length; with a handle it answers
the handle's buffer length. `setattr` without a size accepts and
changes nothing; with a size on a read-only mount answers `EROFS`;
with a size and a handle resizes that handle's buffer and marks it
dirty; with a size and no handle reads the file, resizes it, stores
it at once, and resizes every open writable handle's buffer so a
later flush cannot restore the old length. `open` computes
writability from the access mode and `O_TRUNC` from the raw flags,
answers `EROFS` for either on a read-only mount, seeds the buffer
empty on `O_TRUNC` (marked dirty) or from a `FuseRead` otherwise, and
allocates a handle number. `read` serves the buffer with bounds
clamped. `write` answers `EROFS` on a read-only mount, `EBADF` on an
unknown or non-writable handle, extends the buffer as needed, and
marks it dirty. `flush` and `fsync` store a dirty buffer
(`FuseWrite`) and clear the flag; an unchanged handle is a no-op.
`release` stores a dirty buffer, ignores any error (a release's error
reaches no caller), removes the handle, and answers ok. Every ask is
made from the FUSE session thread through `tokio::runtime::Handle::
block_on`. `FuseRead` answers map `Present` to the bytes, `Missing`
to empty, and `Error` or an undecodable answer to `EIO`; `FuseWrite`
answers map `Ok` to success and anything else to `EIO`.

`src/main.rs` reads `MOUNTS_ENV` (unset: none; unparseable: the
process ends with `the FUSE mounts would not parse`), makes every
mount before any listener binds (a failure ends the process with
`a FUSE mount could not be made`), and holds the `Mounted` values for
the process's life. Routes `/fuse/read/{channel}` and
`/fuse/write/{channel}` are added to the outside router;
`requests::Kind` gains `FuseRead` and `FuseWrite`; `ws.rs` gains the
two answer handlers.

`src/ask.rs` is new: the one ask-and-take-the-first-message relay,
returning `Bytes` or `Asked::{Encode, Empty, Died}`. The vault's HTTP
relay (`src/vault/mod.rs`, moved from `src/vault.rs`) now delegates to
it, mapping `Encode` to `500` and `Empty`/`Died` to `502`; the FUSE
file uses it directly.

### 3.7 What is not yet done for FUSE mounts

The client-side handling of the `FuseRead` and `FuseWrite` channels
(tags `18` and `19`) does not exist: no code on the caller's side
answers them. The provider-side translation of a request's
`fuse_mounts` into the proxy's `MOUNTS_ENV` does not exist, because
the provider's server handles for the container scopes do not exist
(report 14, "What remains", unchanged). The codex container's
documentation names the FUSE mount as the source of its mounted
`auth.json` (section 8.3); the cc container still describes its
caller-provided credentials as an ordinary mount (section 5).

## 4. `container_proxy` reorganized: a module per path, a tree per prefix

The SDK's `container_proxy` module tree now mirrors the path tree.

| former module | current module | path |
|---|---|---|
| `container_proxy::run_loop` | `container_proxy::agent::run` | `/agent/run` |
| `container_proxy::agent_schema` | `container_proxy::agent::schema` | `/agent/schema` |
| `container_proxy::enqueue` | `container_proxy::agent::enqueue` | `/agent/enqueue` |
| `container_proxy::dequeue` | `container_proxy::agent::dequeue` | `/agent/dequeue` |
| (new) | `container_proxy::agent::register` | `/agent/register` |
| `container_proxy::filetree` | `container_proxy::filesystem::tree` | `/filesystem/tree` (was `/filetree`) |
| `container_proxy::read` | `container_proxy::filesystem::read` | `/filesystem/read` (was `/read`) |
| `container_proxy::write` | `container_proxy::filesystem::write` | `/filesystem/write` (was `/write`) |
| (new) | `container_proxy::fuse::{read, write}` | `/fuse/read/{channel}`, `/fuse/write/{channel}` |

`container_proxy::agent` retains `PORT_VARIABLE`, `DEFAULT_PORT`,
`port()`, `enqueue::Fate` and `dequeue::Outcome`. The root keeps
`requests`, `mcp`, `vault`, `command`, `postgres`. The executors moved
with their modules; their error `Display` strings name the new paths.
The `IGNORE_ENV` for the filetree is now
`container_proxy::filesystem::tree::IGNORE_ENV`.

The proxy crate's modules follow: `src/filesystem/{tree/, read.rs,
write.rs, mount.rs}` replace `src/filetree/`, `src/read.rs`,
`src/write.rs`; the routes are `/filesystem/tree`, `/filesystem/read`,
`/filesystem/write`; the handler names in `ws.rs` are
`filesystem_tree`, `filesystem_read`, `filesystem_write`.

## 5. The cc, hermes and openrouter containers

Beyond registration (section 2.4), these three crates are unchanged.
Their documented rules from report 14 stand: no proxy contact before a
request; one run at a time (openrouter's claim captured into the SSE
stream; cc's and hermes's three-phase claim released after the
settlement); the continuation in the caller's database; the first-item
rule; the stream never carrying an error event. The hermes
`HARNESS.md` records the `/register` route and the `409 unregistered`
refusal.

## 6. The pi container is deleted

`diverge-agentic-loop-pi` (`Cargo.toml`, `Containerfile`,
`src/main.rs`; 216 lines) is removed from the tree, from the
workspace members, and from `version.sh`'s `CARGO_TOMLS`. The commit
message states it is unsupported for now.

## 7. The eliza container, built

`diverge-agentic-loop-eliza` is rebuilt in full from a non-compiling
skeleton. `reports/4.md` is the design; `HARNESS.md` is the ruling
where the two differ. The positions below are `HARNESS.md`'s.

### 7.1 The agent, owned by the crate

`src/agent/`: `Agent { character, provider, embedding: Option,
toolsets, memory, plugins: Vec<Plugin> }`, every type deriving
`JsonSchema`; `GET /schema` is `schemars::schema_for!(Agent)`. The
SDK's reference `eliza` module is deleted. Changes against that
reference: `upstream` is dropped; `Provider` is `{ base_url, model }`
and `Embedding` is `{ base_url, model, dimensions }`, both without an
`api_key` (the bearer is the vault's `OPENAI_API_KEY` and
`EMBEDDING_API_KEY` respectively); `Toolsets.web_search` is
`Option<bool>` with the key the vault's `TAVILY_API_KEY`; `plugins`
is new. `Plugin { package: String, settings: Map<String, Value>,
secrets: Vec<Secret> }`. `Secret` is untagged: a bare string is a
static vault key; `{ key, rotates }` names a rotating one. `Rotates`
is untagged: `true` (the same setting), `{ setting }`, `{ file:
[components] }`, or `{ vault: entry }` (Eliza's own vault, `{agent_id}`
substituted). `Character` and `Memory` are as the reference had them.
Absent by ruling: `character.plugins`, `templates`, model tiers,
connectors, cloud and host plumbing, wallets, life-ops, orchestration,
local inference, `settings`/`secrets` maps on the character, and any
credential.

### 7.2 The continuation is the database

Eliza's whole state lives in the caller's database from the first
turn, through `@elizaos/plugin-sql` speaking Postgres at
`POSTGRES_URL = postgres_url()`. Nothing is restored before the
runtime starts and nothing is harvested after. The harness keeps one
row: `eliza_lineage (id smallint PRIMARY KEY CHECK (id = 1), agent_id
uuid NOT NULL, room text NOT NULL, embedding_dimensions integer,
plugins jsonb NOT NULL)`. `agent_id` is minted (v4) on the first run
and pinned into the runtime constructor after; `room` is the constant
`diverge` (entity `diverge:user`); `embedding_dimensions` records the
width the vectors were built at and a run naming another width is
told so by a non-fatal notification; `plugins` records each caller
plugin with its registry-resolved version, which later runs install
regardless of the agent's spec. The row is read at the top of every
run, cached after the first, and written before the runtime starts on
the first run and whenever the plugin set or width changes.

### 7.3 Settings and secrets

Every setting and secret is rendered into the runtime constructor's
settings map and into the entry process's environment, because
`runtime.getSetting()` never reads `process.env` while some plugins
read only that. Nothing is placed on the character's `settings` or
`secrets`. The constructor map takes precedence over the persisted
`agents` row for every key it names. Rendering: `OPENAI_BASE_URL`,
the five text tiers and `OPENAI_IMAGE_DESCRIPTION_MODEL` all set to
`provider.model`, `OPENAI_API_KEY` from the vault; with an embedding,
`plugin-embeddings` loaded with `EMBEDDING_BASE_URL`, `EMBEDDING_MODEL`,
`EMBEDDING_DIMENSIONS`, `EMBEDDING_API_KEY` from the vault, and
without one no embedding tier at all; toolsets to their plugins
(`coding_tools`: `CODING_TOOLS_WORKSPACE_ROOTS=/`,
`SHELL_ALLOWED_DIRECTORY=/`; `documents`: `enableDocuments`,
`DOCUMENTS_PATH=/documents`, `LOAD_DOCS_ON_STARTUP=true`;
`web_search`: `TAVILY_API_KEY`; `generate_media` off unregisters the
`GENERATE_MEDIA` action after `initialize()`); memory switches to the
constructor options `advancedCapabilities` and `enableRelationships`
and the character flags `advancedMemory` and `advancedPlanning`; each
caller plugin's settings as strings; the vault's values inserted last.
Process-only environment: `ELIZA_STATE_DIR=/var/lib/eliza`,
`ELIZA_VAULT_PASSPHRASE`, `ELIZA_VAULT_DISABLE_KEYCHAIN=1`,
`SECRET_SALT`, `ELIZA_DISABLE_TRAJECTORY_LOGGING=1`; `NODE_ENV` is
never set. `plugin-openai`'s `TEXT_EMBEDDING` handler is deleted by
the entry, and its `init` is dropped unless `generate_media` is on.

Secrets come only from the vault. Static: one `get`. Minted by the
harness: `ELIZA_VAULT_PASSPHRASE` and `SECRET_SALT`, read from the
vault or minted (two v4 UUIDs, 64 hexadecimal characters) and set,
stable thereafter; the salt is stable because `initialize()` merges
the persisted row back and a foreign salt fails decryption. Rotating:
`vault_lock` for 300 seconds re-locked every 100 on a task, `get`,
read-back after every turn (including a failed one) from the place
`rotates` names, `set` when the value differs from the last set or
read, `unlock` at the end with every key attempted and the first
failure reported. Read-back channels: `getSetting` through the entry,
a file read by the harness, or `@elizaos/vault` through the entry.
Never the database. A key never read back is a non-fatal notification
at the run's end.

### 7.4 The entry and its protocol

`node/entry.mjs` drives `@elizaos/core` directly; the `@elizaos/agent`
host is not used. One process per run, `cwd=/opt/diverge/eliza`,
stdin and stdout the harness's, stderr passed through. One JSON object
per line discriminated by `type`. Harness to entry: `configure`
(`agentId`, `character`, `settings`, `plugins`, `installed`,
`openaiMedia`, `advancedCapabilities`, `enableRelationships`,
`enableDocuments`, `mcpUrl`), `turn {text}`, `read {kind:
setting|vault, key}`, `stop`. Entry to harness: `fatal {error}`,
`ready`, `text {delta}`, `tool_call {id, name, arguments}`,
`tool_result {id, result}`, `usage {prompt, completion, total}`,
`notification {message}`, `done {text, streamed, failure}`, `value
{key, value}`, `stopped`. The Rust side is `src/run/protocol/`.
`runtime.stop()` runs in its ordinary mode and is awaited without
limit. The runtime is constructed with `checkShouldRespond: false`,
`logLevel: "warn"`; identity is `entityId = createUniqueUuid(runtime,
"diverge:user")`, `roomId = createUniqueUuid(runtime, "diverge")`,
with one `ensureConnection` of type `DM` before ready. A turn is one
`handleMessage` with an abort signal and `onStreamChunk`; the result's
`terminalFailure` or a throw is the `done`'s `failure`. An abandoned
run kills the entry with the stream.

### 7.5 The chunk conversion

Text deltas become `assistant_text_content`; the final text is emitted
only when no delta arrived. The diverge plugin's calls become
`assistant_tool_call` (plugin-minted id, the MCP tool name, JSON
arguments) and `tool_response` with the MCP `CallToolResult`
verbatim; the runtime's own envelopes for `DIVERGE_*`, `REPLY`,
`IGNORE` and `NONE` are dropped; other actions' envelopes convert with
Eliza's `toolCall.id` and a one-text-block result, `isError` from
`status === "failed"`. `evaluation` and `context_event` become
non-fatal notifications. Usage comes from `EventType.MODEL_USED`, one
`usage` chunk per event; a call without counts or a turn without any
event is a non-fatal notification. A delivered queue message is a
`user` chunk. `done.failure` is a fatal notification and ends the run.

### 7.6 The queue, the claim, the server

The queue is taken at each turn's end (nothing steers `handleMessage`
mid-way): pending messages are answered `delivered`, emitted as `user`
chunks, and joined with a blank line into the next turn's text; an
empty queue closes in the same lock hold and ends the run. The claim
is cc's three-phase claim inside a `Teardown` captured into the
stream. `main.rs` is hermes's server: `/register`, `/run` (`409
unregistered`, `409 busy`, `400` empty prompt, `500` database, `500`
for the run's one `Err`, `empty_run` for a run with nothing to say),
`/schema`, `/enqueue`, `/dequeue`.

### 7.7 The diverge plugin: tools and resources, live

`node/diverge.mjs` connects an MCP `Client` over
`StreamableHTTPClientTransport` at `mcp_url()` before the runtime is
constructed, lists tools and resources (paginated), and registers one
native `Action` per tool: name `DIVERGE_` plus the tool name
upper-cased with every character outside `[A-Z0-9]` replaced by `_`
(collisions take a numeric suffix), one `ActionParameter` per
`inputSchema.properties` entry, `validate` always true, and a handler
calling `callTool`. `plugin-mcp` was rejected for paraphrasing every
result through a model call, exposing one untyped action, and an SSRF
guard that may refuse the loopback.

`ActionResult` is `{ success, text }` and nothing else: the planner
JSON-stringifies the whole object into the model's tool result and
there is no attachment part on that path at the pin. Non-text content
is rendered into `text` as Eliza's own ingress renders an attachment:
`image` through `describeImageCached`; `audio` through
`useModel(TRANSCRIPTION, bytes)`; embedded or read `resource` text
with a `Resource <uri> (<mime>):` heading; blobs by MIME type (image
and audio as above, `text/*` and `application/json` as UTF-8,
`application/pdf` through `unpdf`'s `extractText`, otherwise a
`[binary …]` placeholder); `resource_link` as a line naming
`DIVERGE_READ_RESOURCE`. A transcoder that throws yields a placeholder
in place. The `tool_result` line, and so the `tool_response` chunk,
stays the MCP result verbatim.

The lists are live: the plugin subscribes to `tools/list_changed` and
`resources/list_changed`. A tool change re-lists and diffs by name
(removed tools unregistered, new ones registered, changed ones
re-registered under the same action name) and notifies the caller
non-fatally as `{kind: "tools_changed", added, removed, changed}`; a
resource change refreshes the `DIVERGE_RESOURCES` provider's cache and
notifies `{kind: "resources_changed", added, removed}`. Refreshes run
one at a time per list; a failed re-list keeps the previous set and
says so. A notification before the runtime exists marks the list
dirty, and the entry calls `flush()` after `initialize()`. A change
takes effect on the next model call. `DIVERGE_RESOURCES`
(`alwaysInResponseState`) renders one line per resource into every
turn's state. `DIVERGE_READ_RESOURCE` (its name reserved first) takes
one required `uri`, calls `readResource`, and is reported as a
`tool_call` named `read_resource` and a `tool_result` of embedded
resource blocks. Resource templates are not listed.

### 7.8 Plugins the caller names

`src/plugins.rs` installs every named package at `POST /run` before
the runtime starts with `bun add --ignore-scripts <spec>` in
`/opt/diverge/eliza`, at the lineage's pinned version where one is
recorded, records the resolved version from the installed manifest,
and caches installs by spec for the program's life. This is the one
action a run takes before touching the proxy, and it reaches the npm
registry, not the proxy. An unresolvable spec or a package that
exports no plugin refuses the run.

### 7.9 The image and the manifest

`Containerfile`: a Debian `rust:1.98.0` builder (cc's digest), copying
only the eliza crate's manifest and `src` (not `reports/` or
`sources/`); `oven/bun:1.3.14` by digest for the `bun` binary;
`node:24-bookworm-slim` by digest; `ca-certificates`, `git`,
`python3`; the Node project at `/opt/diverge/eliza` installed at build
with `bun install` (scripts allowed); `/var/lib/eliza` and `/documents`
created; the harness binary as entrypoint; `WORKDIR /`. No lockfile
is committed. `node/package.json` pins `@elizaos/{core, plugin-browser,
plugin-coding-tools, plugin-documents, plugin-embeddings, plugin-openai,
plugin-sql, plugin-web-search, vault}` at `2.0.3-beta.7` (verified
published on 2026-09-09), `@modelcontextprotocol/sdk ^1.29.0`, `unpdf
^1.4.0`. `.gitignore` excludes `/sources/`. Cargo dependencies: axum,
the two SDKs, schemars, async-stream, futures-util, rmcp
(client-only), serde, serde_json, sqlx (postgres, rustls), uuid (v4),
tokio (fs, io-util, net, process, rt-multi-thread, sync, time).

`HARNESS.md` enumerates eleven facts a live run must establish,
beginning with the install of the nine pinned packages and the proxy's
pgwire accepting Eliza's pool beside the harness's connection.

## 8. The codex container, new

`diverge-agentic-loop-codex` is a new workspace member. Its facts
about Codex were read from OpenAI's documentation and from
`codex-rs` at tag `rust-v0.153.4` on 2026-09-09; `@openai/codex`
0.153.4 is the pinned version.

### 8.1 The agent

`Agent { model: String, effort: Option<Effort>, reasoning_summary:
Option<Summary>, verbosity: Option<Verbosity>, web_search:
Option<WebSearch>, provider: Option<Provider { base_url }> }`, every
type deriving `JsonSchema`. `Effort` is `minimal | low | medium | high
| xhigh`; `Summary` is `auto | concise | detailed | none`; `Verbosity`
is `low | medium | high`; `WebSearch` is `disabled | cached | indexed |
live`, absent meaning `disabled`. The reference type's `upstream` and
its login field are removed. Absent by ruling: sandbox and approval
settings (the harness always bypasses both), MCP servers other than
the proxy's, session persistence flags, reasoning visibility flags,
images and output schemas, profiles and hooks, and any credential.

### 8.2 The run

One process per turn: `codex exec --json
--dangerously-bypass-approvals-and-sandbox --skip-git-repo-check
[resume <thread_id>] -` with the prompt on stdin, `CODEX_HOME=/root/
.codex`, stderr passed through, `kill_on_drop`. `config.toml` is
written before every run: `model`, `model_reasoning_effort`,
`model_reasoning_summary`, `model_verbosity`, `web_search`,
`mcp_servers.diverge = { url = mcp_url() }`, `hide_agent_reasoning =
false`, and, with a provider, `model_providers.diverge = { name,
base_url, env_key = "OPENAI_API_KEY", wire_api = "responses" }` with
`model_provider = "diverge"`. The queue is taken at the turn's end
(hermes's shape). Before the first event of the first process a
failure is `500`; after, a fatal notification, and the run still
harvests.

### 8.3 Authentication

The agent says nothing about the login. At each run's start the
harness looks, in order: a mounted `$CODEX_HOME/auth.json`, which the
caller serves through a FUSE mount on the container request and the
harness never reads (found, the vault is not consulted); then the
vault's `OPENAI_API_KEY`, placed in the process environment. Both
absent refuses the run naming both. The vault's `OPENAI_CODEX_OAUTH`
document is not a source, by ruling: a login that refreshes is the
caller's to serve live through the mount, and the harness writes no
`auth.json` of its own.

### 8.4 The response vocabulary and the converter

`src/response/` types `codex exec --json`'s output strictly: eight
events (`thread.started`, `turn.started`, `item.started`,
`item.updated`, `item.completed`, `turn.completed`, `turn.failed`,
`error`) and nine items (`agent_message`, `reasoning`,
`command_execution`, `file_change`, `mcp_tool_call`,
`collab_tool_call`, `web_search`, `todo_list`, `error`), with no
catch-all except the source's own `WebSearchAction::Other`. Facts
recorded from the source: no deltas; agent messages and reasoning
arrive whole at `item.completed`; only `todo_list` is updated; an
interrupted turn ends with neither terminal event; the `error` event
is non-terminal and `turn.failed` follows; `turn.completed.usage` is
cumulative per thread; MCP result content is kept as raw JSON blocks.
The converter mints v4 UUID call ids per item, maps each item kind to
the chunk vocabulary (agent messages and reasoning whole; commands,
file changes, MCP calls and web searches as call and response pairs;
todo lists, collab calls and error items as non-fatal notifications;
`turn.failed` as a fatal notification; `turn.completed` as `usage`
computed as the delta against the thread's baseline, clamped at zero,
with a cumulative smaller than the baseline treated as counting from
zero).

### 8.5 The continuation

Two tables: `codex_thread (id smallint PRIMARY KEY CHECK (id = 1),
thread_id text NOT NULL, usage jsonb NOT NULL)` and `codex_files (path
text PRIMARY KEY, content bytea NOT NULL)`. The thread row is read
once per program and cached; the files are restored under
`$CODEX_HOME` once per program; every run harvests at its end, the
fatal end included, every file under `sessions/` and
`archived_sessions/` whose name carries the thread id, as bytes, in
one transaction. Paths are validated before a write. Nothing else
travels.

### 8.6 The image and the manifest

`Containerfile`: the stock Debian `rust:1.98.0` builder; `debian:sid`
by digest (cc's); `ca-certificates`, `curl`, `git`, `nodejs`, `npm`,
`python3`, `python3-pip`, `python3-venv`; `PIP_BREAK_SYSTEM_PACKAGES=1`;
`npm install -g @openai/codex@0.153.4` at build (Apache-2.0); the
harness as entrypoint. Cargo dependencies are eliza's plus `toml 0.8`
and without tokio's `time`. `HARNESS.md` lists nine facts a live run
must establish.

## 9. The python container, new

`diverge-agentic-loop-python` is a new workspace member: openrouter's
server, claim, queue, continuation and history verbatim, with the
model call replaced by one execution of the agent's Python per turn.
The execution contract replicates the CLI's harness
(`objectiveai-daemon/src/python.rs`) on a real interpreter.

### 9.1 The agent

`Agent { python: String, requirements: IndexMap<String, Version> }`;
`Version { operator: Operator, version: String }`; `Operator` is the
seven PEP 440 operators (`==`, `!=`, `>=`, `>`, `<=`, `<`, `~=`),
`===` excluded. `python` is never trimmed. Dropped from the SDK's
reference type: `upstream`, `memory`, `disk`. Every type derives
`JsonSchema`.

### 9.2 Registration is the install

`POST /register` writes `/var/lib/diverge-python/harness.py` (carried
in the binary by `include_str!`) and `agent.py` (the source
verbatim), then, if there are requirements, runs `python3 -m pip
install --no-cache-dir --break-system-packages <name><op><version>…`
and waits without limit, and only then holds the agent. A failed
install is `400 {"kind":"requirements","error":{"status","output"}}`
with pip's stdout and stderr, leaving the agent unregistered; a
disk or pip start failure is `500`; a second registration is `409`.
The handler runs under one `tokio::sync::Mutex` so two registrations
cannot interleave. The interpreter is the system's, with no virtual
environment, by ruling.

### 9.3 The harness and the run

Per turn: `python3 harness.py agent.py`, cwd `/root`, the image's
environment, stdin one JSON object `{input, tools, resources}`,
stdout and stderr captured whole, no timeout. The feed is written and
the process awaited concurrently; the harness reads stdin to its end
before writing anything. The harness parses the source; if the last
statement is a bare expression it is split off, the rest is `exec`'d
and it is `eval`'d, otherwise the whole is `exec`'d; one globals
dictionary with `__name__ == "__main__"` and the names `input`,
`tools`, `resources`; the script's prints are captured; the last line
of the real stdout is `{"eval": <value or null>, "stdout":
<captured>}`; then `flush` and `os._exit(0)`. No safeguards against a
script that breaks the envelope, as in the CLI. No base64.

Classification, in the CLI's order: nonzero status or signal is
`exception {status, stderr}`; a clean exit whose last non-empty line
is not the envelope is `harness {line, error}`; `eval` not `null` is
the value, else the printed text trimmed and read as JSON (`printed`
if not JSON, `no_output` if empty); a value that is not a list of
chunks is `deserialize {path, error}`; a chunk the script may not emit
is `forbidden {index, chunk}`. All under `{"kind":"python","error":…}`.

### 9.4 What the script may emit and what it is fed

The output is a JSON array of `AgenticLoopChunk`s restricted to the six
assistant kinds and `notification`; `user`, `usage` and
`tool_response` are refused and one such chunk refuses the whole
output. An empty array is a valid turn that said nothing. The last
expression wins over printed output when it is anything but `None`.
`input` is the continuation in its stored form (chunk objects and bare
prompt strings, this turn's prompt or the delivered messages last).
`tools` and `resources` are rmcp's `Tool`s and `Resource`s, re-listed
before every invocation of the script, never cached.

The harness was executed on this host's CPython 3.13 with three
fixture scripts and confirmed: a last-expression list is returned in
`eval`; a printed list is captured in `stdout` with `eval` null; an
uncaught exception produces a traceback on stderr and exit status 1;
a script inspecting `input[-1]` distinguishes a first turn from a
turn after a tool response.

### 9.5 The loop

Every `assistant_tool_call` in the output is a call (fragments
coalesced by id); since the script cannot emit a tool response, the
whole output is the last segment. Before anything of the turn is
emitted, every call's name is checked against the tool list the script
was fed; a name absent from it is `{"kind":"loop","error":{"kind":
"unknown_tool","id","name"}}` and ends the run, a `500` on the first
turn and a fatal notification thereafter. The calls are then spawned
on their own tasks before the turn's chunks are yielded, and each
response is yielded and recorded in completion order; arguments that
are not a JSON object are sent as none; a transport failure is fatal.
No in-process reach back into the container exists (no analogue of
the CLI's `objectiveai.execute`). The seams, the history and the queue
are openrouter's unchanged: `take()` after a turn's responses, then a
rest; a call-less turn that spoke rests first, then `take_or_close()`.
No `usage` chunk is produced. Nothing times out.

### 9.6 The image and the manifest

`Containerfile`: openrouter's `rust:1.98.0-alpine3.24` builder by
digest (a static musl binary); `python:3.13-slim-bookworm` by the tag
manifest digest published 2026-09-01; `/var/lib/diverge-python`
created; `WORKDIR /root`; the binary as entrypoint. Cargo
dependencies: openrouter's minus `mime2ext`, `rust_decimal`,
`reqwest`, `reqwest-eventsource` and `eventsource-stream`, with tokio
gaining `fs`, `io-util` and `process`. `HARNESS.md` records the
settled decisions.

## 10. Workspace and tooling

`Cargo.toml` members: `diverge-agentic-loop-codex` and
`diverge-agentic-loop-python` added, `diverge-agentic-loop-pi`
removed. `Cargo.lock` updated for the two new crates and the proxy's
`fuser` and `libc`. `version.sh`'s `CARGO_TOMLS` list: `pi` removed.
It does not list `diverge-agentic-loop-codex` or
`diverge-agentic-loop-python`, both at `2.3.0`; a version bump run
through the script would leave those two behind.

## 11. What remains

Unchanged from report 14: the provider's server handles for the three
container scopes, with the loopback OCI registry and digest store
behind them; the spec site's prose; a live run of anything.

Added by this range: the caller-side handling of the `FuseRead` and
`FuseWrite` channels; the provider-side rendering of `fuse_mounts`
into `MOUNTS_ENV`; the cc container restated to obtain its credential
file through a FUSE mount; the removal of the
`codex` and `python` reference modules from the SDK now that both
images own their types; `version.sh` entries for the codex and python
crates; image builds and live runs of the eliza, codex and python
containers, for which each `HARNESS.md` enumerates the facts to be
established.
