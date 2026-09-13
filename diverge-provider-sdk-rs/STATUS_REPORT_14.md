# Report 14: the containers API, the proxy, and three containers

Report 12 described a world with three container endpoints, a
container surface that was a WebSocket on the container's own port, a
queue that lived in an MCP proxy, resources fetched by identity and
a continuation carried as bytes on the wire. Report 13 designed the
OCI pull. None of what stood then stands now. This report is the
whole differential from report 12's state to the current one, as it
is settled — not the steps between. Nothing here has been built as an
image or run live; everything checks with zero warnings under every
feature combination and documents clean.

## What is gone

- The three endpoints `agentic_loop`, `laboratories` and
  `mcp_plugin`, with their client proxy traits, the
  `server::laboratories` registry and the two legacy tags.
- The `agentic_loop_container`, `mcp_proxy` and `postgres_proxy`
  modules, and the crates `diverge-container-mcp-proxy` and
  `diverge-container-postgres-proxy`.
- The container surface as a WebSocket at `/` on `14978`, and with it
  the `Response` vocabulary that rode it: `FetchContinuation`,
  `FetchResource`, `Continuation`, `Resource`.
- Resources — rotating credentials fetched by identity and echoed
  back — and the continuation as bytes on the stream.
- The queue in the proxy, and the fold onto a tool response with its
  SHA-256 correlation key.
- `shared::oci`, the tunnel; `shared::http` before it.
- The `environment` field on the request; `HashMount` and its
  `f1`/`d1` identity prefixes.
- The typed agent enum on the wire, and the `Openrouter`, `ClaudeCode`
  and `Hermes` variants of the reference enum that replaced it.
- Nothing listens on `14978`, and no image ships a proxy.

## The `containers` endpoint

One substrate — a provider runs a container for a caller: an image,
limits, the caller's mounts — and two families told apart by what the
caller reaches in for:

- **agents**: the image runs an agentic loop. The request carries a
  prompt and an agent, a JSON value the image defines. The caller
  starts the loop over a channel that carries nothing and reads its
  chunks back; asks what the agent value may be; and adds to, or
  clears, the running loop's queue.
- **tools**: the image runs an MCP server. The caller opens the five
  MCP exchanges into it.

Three scopes: the agents' `run`, the tools' `run`, and the tools'
`connect` that joins a running tool container by id and
authorization. There is no agents connect: a loop has one caller, the
one that gave it its prompt. The endpoint tags are now:

| tag | request |
|---|---|
| `0` | `containers::agents::run` |
| `1` | `containers::tools::run` |
| `2` | `containers::tools::connect` |
| `3`–`7` | `volumes::{list, watch, create, edit, delete}` |
| `8` | `images::check` |
| `9` | `version` |

The main stream is quiet. A run answers with the container's `Id`
and then nothing for as long as the container runs; an error then a
finish means it never came up or is gone; a finish with no error is
the run over. A connect answers nothing at all while attached, one
`Error` newtype then a finish when it never was. Everything that used
to ride channel `0` is a channel the caller opens, so a caller that
wants none of it pays for none, and two callers on one container each
subscribe to what they want.

## The request

`shared::containers::request::Container`: `image`, `memory`, `disk`,
`volume_mounts`, `file_mounts`, `directory_mounts`. An `Image` is
either one a provider can reach or `Client { name, digest }`, one the
caller holds. A `VolumeMount` is `{host_name, host_relative_path,
container_path}`; an `IdentityMount` is `{container_path, identity}`
where the identity is `<size>:<base64url sha256>` — of the bytes for
a file, of the manifest for a directory, the size riding it so a
provider can judge the weight before fetching. `Connect { id,
authorization }` is the other way to reach a container. The agents
run request is the container plus `prompt: String` and `agent:
Value`. The answer to a run is `response::Id`.

## What the caller opens

The first five tags are the same on every scope, in the same order,
so a reader of one is a reader of all; each family's own exchange
takes the tags after.

| tag | agents run | tools run | tools connect |
|---|---|---|---|
| `0` | `Stop` | `Stop` | `Disconnect` |
| `1` | `Filetree` (bare) | `Filetree` | `Filetree` |
| `2` | `Read` | `Read` | `Read` |
| `3` | `Write` (write_path) | `Write` | `Write` |
| `4` | `Postgres` | `Postgres` | `Postgres` |
| `5` | `RunLoop` (bare) | `McpListTools` | `McpListTools` |
| `6` | `AgentSchema` (bare) | `McpListResources` | `McpListResources` |
| `7` | `Enqueue { prompt }` | `McpCallTool` | `McpCallTool` |
| `8` | `Dequeue` (bare) | `McpReadResource` | `McpReadResource` |
| `9` | — | `McpNotifications` | `McpNotifications` |

Every ask that carries nothing is a bare variant: a request type with
no fields was a type for nothing. What answers each is under
`server::channel_response`, one module per tag, each an alias of the
shared shape.

## What the provider opens

On a run, eighteen; on a connect, only `WriteBytes`.

| tag | ask | answered with |
|---|---|---|
| `0` | `OciManifest` (digest) | one frame: `u16` media-type length, media type, bytes; or empty |
| `1` | `OciBlob` (digest) | bytes chunked at `CHUNK_SIZE`; or empty |
| `2` | `Authorize { address, authorization }` | `Denied` or `Authorized` |
| `3` | `Write` (write_bytes, by write id) | the content, chunked, then the finish |
| `4` | `FetchFile` (identity) | the bytes, chunked |
| `5` | `FetchDirectory` (identity) | one frame per file, or several adjacently |
| `6` | `Postgres { connection_id }` | everything the database says |
| `7` | `Command` (opaque bytes) | one `Item` per item, then the finish, or `Error` last |
| `8`–`12` | `VaultGet`, `VaultSet`, `VaultDelete`, `VaultLock { ttl }`, `VaultUnlock` | one frame each |
| `13`–`17` | `McpListTools`, `McpListResources`, `McpCallTool`, `McpReadResource`, `McpNotifications` | the MCP result; notifications stream |

## The shared vocabulary, exchange by exchange

All of it is `shared::containers`, defined once and wrapped or
aliased by each scope.

- **read**: one file, never a directory, because nothing walking a
  filesystem from userspace can snapshot it. One frame of bytes.
- **write**: two exchanges, because only a responder can end a
  channel. The caller opens `write_path { write_id, path }` and gets
  ok or error once the file has landed; the provider opens
  `write_bytes { write_id }` and the caller streams the content until
  it finishes. A read pipes into a write without buffering.
- **filetree**: the caller's channel is the ask. Snapshot then
  deltas over the container's root, one frame per changed node,
  fresh per channel. `Frame` is `Snapshot`, `Inserted`, `Modified`,
  `Removed` — there is no move: a rename is a removal and an
  insertion. `Node::Directory` carries `changes: bool`, `false` for a
  subtree the source could not watch, which is then possibly stale
  until the next snapshot. An overflow is a second snapshot
  in-stream. What a provider leaves out is its own to decide.
- **postgres**: every connection the container dials is a pair of
  channels correlated by `connection_id` — the provider's carries
  what the database says, the caller's carries what the container
  wrote; either finish is that side hung up, an empty finish the
  caller declining.
- **command**: the bytes are opaque, the CLI's vocabulary, not the
  wire's. Items then the finish, or `Error` last.
- **vault**: a key-value store with locks, living with the caller;
  keys are the container's, the namespace the caller's. A lock is
  answered when held, its TTL runs from the grant, the container is
  the holder across connections, locking again refreshes, expiry
  releases silently, `0` is refused. No retry on any vault operation.
  New: `vault::keys`, well-known names any image may share —
  `NOUS_OAUTH`, `OPENAI_CODEX_OAUTH`, `MINIMAX_OAUTH`, `QWEN_OAUTH`,
  `SPOTIFY_OAUTH` — each the whole JSON document of that login, and
  the cycle an image owes one: lock, get, run, set, unlock.
- **mcp** (`shared::mcp`): the five exchanges, four unary and
  notifications streamed, shared because they ride every scope in
  more than one direction.
- **oci**: report 13, landed. `manifest` and `blob`, by digest. The
  provider runs its own loopback registry and digest store; the
  caller keeps a store and needs no HTTP.
- **authorize**: the runner is asked whether a connector may join —
  its address and what it offered — before the connect scope opens.
- **fetch_file**, **fetch_directory**: the mounted content the
  provider does not hold, by identity.
- **run_loop**: `Frame::Chunk(AgenticLoopChunk)` or `Frame::Error`,
  then the finish. The chunk vocabulary is unchanged from report 12
  except that nothing carries a continuation any more.
- **agent_schema**: `AgentSchema(Value)` or `Error`, once.
- **enqueue**: `{ prompt }`, answered when the fate is known —
  `Delivered`, `Dequeued`, `Missed`, or `Error` — and never timed out.
- **dequeue**: bare, answered `Dequeued`, `Empty` or `Error`; every
  message withdrawn is also answered on its own enqueue channel.

`CHUNK_SIZE` is one constant at the crate root, 2 MiB, used by every
chunked exchange.

## The container proxy

`diverge-container-proxy` is one program the HOST injects into every
container at runtime — it is in no image, and it may not be up until
a request comes. It is the container's whole way to the caller's
world and the caller's whole window into the container, and
`container_proxy` in this crate is the wire it speaks. Two listeners,
one per audience, so no path has to say which side it faces:

**`OUTSIDE_PORT`, `14979`, every interface — the provider's server
dials it.** `/requests` is one WebSocket carrying the container's
asks as frames `[channel: u32][kind: u8][payload]` and nothing back:

| kind | ask | answered on |
|---|---|---|
| `0`–`3` | MCP list-tools, list-resources, call-tool, read-resource (params JSON) | `/mcp/<exchange>/{channel}` |
| `4` | MCP notifications | `/mcp/notifications/{channel}` |
| `5`–`9` | vault get, set, delete, lock, unlock | `/vault/<op>/{channel}` |
| `10` | command (opaque) | `/command/{channel}` |
| `11` | postgres (a connection announced) | `/postgres/{channel}` |

The server answers by opening the ask's own path with the channel in
it, sending raw messages — the response type and nothing around it —
and closing: a clean close is the answer complete, a close with no
message is could-not-serve, an abrupt end is the answer dead. An
unknown channel is `404`, a second opening `409`. `/postgres/{channel}`
is a raw bidirectional pgwire conduit. Then the paths the server
opens on its own: `/filetree` (as many as it likes; the proxy watches
from `/` with one inotify watcher per connection, ignoring `/proc`,
`/sys`, `/dev` and the mounts the server names in
`DIVERGE_CONTAINER_PROXY_FILETREE_IGNORE` as a JSON array of
component arrays, unparseable meaning empty), `/read` (one request,
the bytes then the close, or an error string), `/write` (the request,
chunks, an empty message as the end, ok or error; landed via a
sibling temp file and rename), and the four `/agent/*` paths below.

**`INSIDE_PORT`, `80`, loopback — the program inside dials it.**
`/mcp` is a fully compliant Streamable HTTP MCP server whose every
exchange becomes an ask on `/requests`, its notifications gated
behind the agent's first exchange; `/vault/<op>` is the vault as
plain HTTP, one ask per call; `/command` is a command posted and its
items streamed back. **`postgres::LOOPBACK_PORT`, `81`**, is the
pgwire listener the container's own driver dials, each connection one
ask. The proxy binds all three or none.

The rules every path shares: every message is one binary frame; the
server dials; there is no handshake; a clean close and an abrupt end
are told apart; nothing times anything out. What dying means is
stated once per kind: MCP exchanges are re-asked on the next
connection, at-least-once; a vault operation or a command is reported
failed and never repeated; a filetree starts over; a read or write is
that one file failing.

## The agent's own server

An agent container's entrypoint is an HTTP server on the loopback at
`container_proxy::agent::port()` — `PORT` from the environment when it
parses as a port in `1..=65535`, `8080` otherwise — and the proxy
FORWARDS to it, dialing only when the provider's server has opened a
path that needs it. The proxy cannot tell an agent container from a
tool container; the server can, and only opens these on an agent
container. The proxy caches nothing and hosts no attachment point.

| the proxy's path (outside) | it calls (inside) | the agent's server answers |
|---|---|---|
| `/agent/run` | `POST /run`, body the `{prompt, agent}` JSON | `2xx` as `text/event-stream`, every `data:` one chunk JSON, the stream's end the loop ended; or a non-`2xx` |
| `/agent/schema` | `GET /schema` | `2xx` the JSON Schema; or a non-`2xx` |
| `/agent/enqueue` | `POST /enqueue`, the `{prompt}` JSON | `2xx` `{"type":"delivered"|"dequeued"|"missed"}`, held until known; or a non-`2xx` |
| `/agent/dequeue` | `POST /dequeue`, `{}` | `2xx` `{"type":"dequeued"|"empty"}`; or a non-`2xx` |

The stream never carries an error event. The agent's server answers
either a non-`2xx` whose JSON body IS the `Error` frame's value —
wrapped as `{"kind":"agent","status":N,"error":text}` when it is not
JSON — or a `2xx` stream of chunks in which any later failure is a
fatal notification chunk. The first item decides which side of that
rule a failure is on, and every container pulls it before choosing a
status. A dial the agent's server refuses is `Error` at once. The
proxy re-frames each event as `[0][data]` without reading it; the
`CHUNK` and `AGENT_SCHEMA` tags are public for that. Between runs
the queue verbs still answer: an enqueue with no loop running is
`missed`. `container_proxy::{agent, enqueue, dequeue}` are new
modules: `agent::{PORT_VARIABLE, DEFAULT_PORT, port, enqueue::Fate,
dequeue::Outcome}`, and the two path modules with executors.

## The server side of this crate

`server::ContainerClient::new(url)` is the one place the crate dials a
proxy: a `ws://host:14979` base, a path appended, the socket
`ContainerWebSocket` — tungstenite's own, since the wire lives on the
difference between a clean close and an abrupt end. Under every
`container_proxy::<path>` an `execute` module, behind the `server`
feature, does what the path does: `requests` yields the container's
asks; the unary MCP and vault paths answer one frame or refuse with
the empty close; notifications, command and postgres hand back a
handle with `send` and `finish`; filetree, read and the agent's run
are streams with a terminal `Refused`; write takes a content stream;
agent_schema, enqueue and dequeue read one answer and drain the close
(enqueue and dequeue return the whole frame, `Error` included, since
the caller's channel takes exactly that). A crate-private `Messages`
adapter reads a socket as binary messages with `None` for the clean
close and `Closed`/`Socket` for the abrupt one. `ContainerDeployer::
client` takes the registry's address and repository for the loopback
registry the provider will run. Not written: the handles that serve
the three container scopes, and the registry and digest store behind
them; dispatch finishes the scopes with nothing.

## The container SDK

`diverge-container-proxy-sdk` is the proxy as one `Client` for a
program inside a container, no port or path named anywhere in it.
Making it is no I/O. `mcp_list_tools`, `mcp_list_resources`,
`mcp_call_tool`, `mcp_read_resource` and `mcp_peer` share one session
opened on first use; `mcp_url()` is the proxy's MCP address for a
program whose MCP client is somebody else's — a subprocess handed a
server list — the one place it is spelled. `vault_get`, `vault_set`,
`vault_delete`, `vault_lock(key, ttl)`, `vault_unlock`; `command_run`
streaming items; `postgres_address()` and `postgres_url()`, the
latter `postgres://diverge@127.0.0.1:81/diverge?sslmode=disable` —
user and database fixed, no password, TLS off, the caller's side the
authority on what the connection reaches. The loop is not this
crate's: an agent container's program is its own server.

## Rules every container keeps

- **Nothing of the proxy's before a request.** The server binds and
  waits; the vault, the database and MCP are all `POST /run`'s. Each
  Containerfile builds the harness alone and exposes nothing.
- **One run at a time, never one per lifetime.** A run beside one
  streaming is `409 busy`; a run after it resumes the conversation.
  Openrouter's claim is a synchronous guard captured into the SSE
  stream, released the instant the stream drops; cc's and Hermes's
  ride inside the run's teardown and are released only after the
  settlement task has cleared the run's locks and decided its fates,
  with a third phase, settling, so a request that lands meanwhile
  waits instead of being refused. Openrouter's queue reopens per run
  with a generation every close quotes, so a close spawned by an
  ended run cannot touch the next.
- **Every container owns its agent.** The types moved with `git mv`
  into the crate that reads them, dropped the `upstream`
  discriminator, and derive `schemars::JsonSchema`, so `/schema` is
  `schema_for!(Agent)` over the type the loop parses.
- **The continuation lives in the caller's database**, reached
  through the proxy's pgwire, each image with its own table.

## Openrouter

The key is the vault's `OPENROUTER_API_KEY`. The continuation is the
conversation's items — prompts as strings, chunks as objects — in
`continuation (id smallint PRIMARY KEY CHECK (id = 1), state jsonb NOT
NULL)`, loaded on the first run, cached in memory after every
successful save, upserted at every point the history is at rest: after
a tool turn's answers all landed, and after a call-less turn that said
something. The loop yields those rests; the server saves them. The
queue is consulted at two seams — after tool answers, and the atomic
last look on a call-less turn, where pending messages open another
turn and only an empty queue ends the loop. Delivered prompts fold
onto a tool response as a `system-reminder` or open a user message,
a run of them joined with a blank line. The image is the loop alone
on Alpine.

## Claude Code

No credential handling at all, by ruling: Claude Code's terms forbid
it, so the caller mounts whatever Claude Code accepts and the
container says nothing about it. Claude Code installs from npm at
startup, the one thing done before a request and not the proxy's.
The continuation is `{session_id, files}` — the session's own files
under `projects/**`, `file-history/<session>/**`, `tasks/<session>/**`,
never the config dir at large — in the same one-row jsonb table,
harvested at every run's end and on a fatal end after the model
spoke. The session id, once known from the row or the first record
naming it, never changes: a later run reads no row, writes no file,
and launches `claude --resume`. The queue is Claude Code's own over
stream-json stdin — replay echoes decide delivered, cancel replies
decide dequeued, the end-of-stream drain decides missed — and the
two-lock-and-a-map design under it is untouched. Claude Code's
`--mcp-config` takes `mcp_url()`. The image is Debian sid, for
skills' scripts.

## Hermes

Hermes cannot be steered mid-turn, so the queue is taken at each
turn's end: pending messages, joined with a blank line, become the
next `/v1/runs` turn on the same session, and invoking Hermes again
is the delivery; an empty queue ends the run. The continuation —
`state.db` folded (`VACUUM`, `wal_checkpoint(TRUNCATE)`) and the two
memory files — leaves as tagged 2 MiB pieces, each one row of
`continuation (seq integer PRIMARY KEY, frame bytea NOT NULL)`,
written in one transaction; restore reads the rows back through the
same ingest, once per program life, then `quick_check`s the database
and names the session; later runs read the tip from the database on
disk. Rotating OAuth logins come from the vault under the well-known
keys, one cycle per run: lock for 300 seconds refreshed every 100 on
a task, get, write to `auth.json` or the Qwen file, run, read back,
set, unlock — every key attempted, the first failure reported, a
document Hermes quarantined not set but still unlocked. The agent's
`*_resource` fields are gone; choosing the provider is the ask. Static
credentials stay arguments in the agent, applied as environment.
Skills are probed on disk under the external skills path. The gateway
is started per run and stopped before the harvest; the MCP entry in
its config comes from `mcp_url()`. The image is the harness alone
inside `nousresearch/hermes-agent`. `HARNESS.md` is rewritten to all
of this.

## What remains

The provider's server handles for the three container scopes, and
with them the loopback OCI registry and digest store. The spec site's
prose for everything above. The eliza and pi containers, still on the
old world. A live run of any of it.
