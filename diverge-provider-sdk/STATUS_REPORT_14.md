# Report 14: the proxy forwards, and three containers come up

Report 13 designed the OCI pull as two typed fetches. Everything
since is here: that design landed, the mount vocabulary tidied, the
proxy's executors written, the agent container's own surface decided
twice — once wrongly, once as it now stands — the proxy split across
two ports, and the openrouter, Claude Code and Hermes containers
brought up to the containers API, each with the same shape and its
own rules. Nothing in this report has been built as an image or run
live; everything in it checks with zero warnings and documents clean.

## The OCI pull, landed

`shared::containers::oci::{manifest, blob}` are the two exchanges
report 13 described. `shared::oci`, the tunnel, is deleted, with the
client-side registry and the OCI stream that served it. On the
server-opened side of an agents or tools run the tags shifted by one
to make room: `OciManifest` is `0`, `OciBlob` `1`, and `Authorize`
through `Mcp*` follow as `2` through `17`. A manifest answers one
frame — a `u16` media-type length, the media type, the bytes — and a
blob answers bytes chunked at `CHUNK_SIZE`; either finishes empty for
a digest the caller does not hold. The provider's own loopback
registry and digest store are still to write; they arrive with the
server handles.

## Mounts, renamed

Three renames, in order, so the vocabulary says the same thing
everywhere: `HashMount` names its path `container_path` as
`VolumeMount` does; then `hash` becomes `identity`, because the
fetches already called it that; then the type is `IdentityMount`, and
the identity is `<size>:<base64url sha256>` with no `f1`/`d1` prefix —
the prefix said nothing the field's own type did not.

## The proxy's executors, behind `server`

`server::ContainerClient::new(url)` is the one place this crate dials
a container proxy: a `ws://host:port` base, a path appended. The
socket type is `ContainerWebSocket`, tungstenite's own, because the
proxy's wire lives on the difference between a clean close and an
abrupt end. Under every `container_proxy::<path>` an `execute` module
answers or asks in the path's shape: `requests` yields the container's
asks as a stream; the unary MCP and vault paths answer one frame or
refuse; notifications, command and postgres hand back a handle with
`send` and `finish`, dropping it being the answer dying; filetree and
read are streams with a terminal `Refused`; write takes a content
stream and answers ok or error. A crate-private `Messages` adapter
turns a socket into its binary messages with the two endings told
apart: `None` for the clean close, `Closed` or `Socket` for the abrupt
one.

## The agents run request carries the prompt and the agent

An agent container's run request is the container plus `prompt` and
`agent`, the agent a JSON value the image defines. Every ask that
carries nothing is a bare variant now — `RunLoop`, `AgentSchema`,
`Dequeue`, `Filetree`, `Stop` — because a request type with no
fields was a type for nothing. `Enqueue` and `Dequeue` are back on the
agents scope, as `shared::containers::{enqueue, dequeue}`: fates
`Delivered`/`Dequeued`/`Missed`/`Error` and `Dequeued`/`Empty`/
`Error`. Only tool containers connect; the agents scope has no
`connect`. A short-lived queue feature in the proxy itself was
investigated and dropped.

## The agent surface, decided twice

The first design made the proxy the loop's emitter: a harness
attached to the proxy at `/run-loop/agent`, posted its schema at
`/agent-schema/agent`, and the proxy cached the schema and
rendezvoused the two sides. That landed and was then rejected, on two
grounds: the proxy must cache nothing, and it must host no attachment
point.

What stands instead: the agent container's entrypoint is an HTTP
server of its own on the container's loopback, and the proxy
FORWARDS to it, dialing it only when the provider's server has opened
a path that needs it — which the server does only on an agent
container, the one party that knows which kind it made. The server is
at [`container_proxy::agent::port()`]: `PORT` from the environment
when it parses as a port, `8080` otherwise. The proxy's paths carry
the `/agent` prefix because they only work on an agent container:

| the proxy's path | it calls | the agent's server answers |
|---|---|---|
| `/agent/run` | `POST /run`, the `{prompt, agent}` JSON | `2xx` as SSE, every `data:` one chunk; or a non-`2xx` |
| `/agent/schema` | `GET /schema` | `2xx` JSON Schema; or a non-`2xx` |
| `/agent/enqueue` | `POST /enqueue` | `2xx` `{"type": "delivered"\|"dequeued"\|"missed"}`, held until known |
| `/agent/dequeue` | `POST /dequeue`, `{}` | `2xx` `{"type": "dequeued"\|"empty"}` |

The stream never carries an error event: the agent's server answers
either a non-`2xx` whose JSON body IS the `Error` frame's value, or a
`2xx` stream of chunks in which a later failure is a fatal
notification chunk. The first item decides which side of that rule a
failure is on, and every container pulls it before choosing a status.
A dial the agent's server refuses is `Error` at once; nothing waits
or retries. `container_proxy::{enqueue, dequeue}` are new path
modules with executors returning the whole frame, `Error` included,
since the caller's channel takes exactly that frame.

## Two ports

The proxy served two audiences on one port, which forced an `/agent`
segment onto the paths the program inside dials and caused one real
bug: a hard-coded `/mcp` where the proxy served `/mcp/agent`. Now:
`OUTSIDE_PORT`, `14979`, bound on every interface, carries every path
of the wire — `/requests`, the answer paths, `/postgres/{channel}`,
`/filetree`, `/read`, `/write`, `/agent/*`; `INSIDE_PORT`, `80`, bound
on the loopback, carries what the program dials — `/mcp`,
`/vault/<op>`, `/command`; `postgres::LOOPBACK_PORT`, `81`, is the
pgwire listener. The proxy binds all three or none and runs its two
routers joined. The container SDK builds on the inside port, and
exports `mcp_url()` so a program whose MCP client is not the SDK's — a
subprocess handed a server list — spells the address nowhere.

## The proxy is injected

No image ships the proxy. The host injects it at runtime, and it may
not be up at all until a request comes, so every container binds its
server and touches nothing of the proxy's — vault, database, MCP —
until `POST /run`. Making the container SDK's `Client` is no I/O.
Each Containerfile builds the harness alone and exposes nothing.

## One run at a time

Every container refuses a run beside one streaming with `409 busy`
and admits a run after it. The lock's life is tied to the run's:
openrouter's claim is a synchronous guard captured into the SSE
stream, released the instant the stream drops; cc's and Hermes's ride
inside the run's teardown and are released only after the settlement
task has cleared the run's locks and decided its fates, with a third
phase — settling — so a request that lands during the settlement
waits rather than being refused. Openrouter's queue reopens per run
with a generation number every close quotes, so a close spawned by a
run that ended cannot touch the run that came after.

## Every container owns its agent

The reference agent module in this crate has lost `Openrouter`,
`ClaudeCode` and `Hermes`: each moved, with `git mv`, into the crate
that reads it, dropped its `upstream` discriminator, and derives
`schemars::JsonSchema`, so `GET /schema` is `schema_for!(Agent)` over
the very type the loop parses. `codex`, `eliza` and `python` remain
for reference.

## Openrouter

An HTTP server on the agent port. The key is the vault's
`OPENROUTER_API_KEY`. The continuation is the conversation's history
as one jsonb row — `continuation (id smallint PRIMARY KEY CHECK (id =
1), state jsonb NOT NULL)` — loaded on the first run, cached in memory
after every successful save, and upserted at every point the history
is at rest: after a tool turn's answers all landed, and after a
call-less turn that said something. The loop yields those rests; the
parent saves them. The queue is the crate's own, consulted at two
seams — after tool answers, and the atomic last look on a call-less
turn, where pending messages open another turn and only an empty
queue ends the loop.

## Claude Code

The same server. No credential handling at all, by ruling: Claude
Code's terms forbid the container handling credentials on its behalf,
so the caller mounts whatever Claude Code accepts and the container
says nothing about it. The continuation is `{session_id, files}` in
the same one-row table, harvested from `projects/**` at every run's
end; the session id, once known — from the row or the first record
naming it — never changes, so a later run reads no row, writes no
file, and launches `claude --resume` with it. The queue is Claude
Code's own, reached through stdin, and the two-lock-and-a-map design
under it is untouched; only its fate types changed. The proxy's MCP
URL reaches Claude Code through `mcp_url()`.

## Hermes

The same server. Hermes cannot be steered mid-turn, so the queue is
taken at each turn's end: pending messages, joined with a blank line
as openrouter joins a run of prompts, become the next `/v1/runs` turn
on the same session, and invoking Hermes again is the delivery. The
continuation — `state.db` and the two memory files — leaves as it
always did, folded and read a piece at a time behind a tag byte, but
each piece is now one row of `continuation (seq integer PRIMARY KEY,
frame bytea NOT NULL)`, written in one transaction; restore reads the
rows back through the same ingest, once per program life. Rotating
OAuth logins come from the vault under fixed well-known keys —
`shared::containers::vault::keys::{NOUS_OAUTH, OPENAI_CODEX_OAUTH,
MINIMAX_OAUTH, QWEN_OAUTH, SPOTIFY_OAUTH}`, shared by any image that
speaks the same login — and every run owes the cycle: lock for 300
seconds, refreshed every 100 on a task, get, write, run, read back,
set, unlock, every key attempted and the first failure reported. The
agent's `*_resource` fields are gone; choosing the provider is the
whole ask. Skills are probed on disk. The image is the harness alone
inside Hermes's own image.

## What remains

The provider's server handles for the container scopes, and with
them the loopback OCI registry and digest store. The spec site's
prose. The eliza and pi containers, still halted. A live run of any
of it.
