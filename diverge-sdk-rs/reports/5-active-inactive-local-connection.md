# Report 5: the loop's begin and end on the wire, and a connection carried in the process

Covers one commit on `p2p-provider-binary`, 2026-10-06, across
`src/container_proxy/outside`, `src/provider/endpoints/containers`
and `src/wire`. Both changes are wire changes; both append.

## Why

The daemon holds an agent's loop to be ACTIVE or not: a delete, an
edit of the mounts and a detach are refused while it is; the idle
clock does not run while it is; a tool container runs while an
attached agent's is; a dependency the deployer agent was asked to
answer is unmet when the deployer's ends first. The wire did not
carry the fact. The run's main stream carried chunks and nothing
else, and the crate said in so many words that quiet was an agent
with nothing left to say — which is true, and is also what an agent
working through a slow model looks like. A caller cannot derive
"active" from silence, and a chunk kind would be the program's to
forge.

The proxy is the one party that knows both moments: a loop is one
`POST /run` on the agent's server, begun when that call answers
`2xx` and ended when its event stream ends. So the proxy says them.

## What changed

| where | what |
|---|---|
| `container_proxy::outside::endpoints::agents::begin::server::response::Frame` | `Active` tag `3`, `Inactive` tag `4`, each the tag byte and nothing after it; `pub const ACTIVE`, `INACTIVE` |
| `provider::endpoints::containers::agents::run::server::response::Frame` | `Active` tag `4`, `Inactive` tag `5`, the same shape; `pub const ACTIVE`, `INACTIVE` |
| `provider::endpoints::containers::agents::run::client::execute::Event` | new: `Chunk(AgenticLoopChunk) \| Active \| Inactive`, the item of both layers' conversation readers |
| `…begin::client::execute::Chunks` | yields `Event` |
| `…run::client::execute::ExecuteStream` | yields `Event`; the two words are items, never read past |
| `…containers::server::relay::chunks` | relays each `Event` as the run frame of the same meaning |
| `wire::connection::Connection` | `Local { incoming, outgoing }`: a connection carried by channels inside the process; `connection::Error::Local` |

The two words ride under tags, not inside chunks, for the reason the
run frame already gave about its error: the byte in front is what
keeps a program's output from being read as a statement about it.
The program's stream is chunks, every one under the chunk's byte, so
nothing it writes is read as either word.

`Connection::Local` exists for the daemon: a container's `/daemon`
connection reaches it as a pair of channels on the run scope, and
the daemon serves it through the same session, scopes and dispatch a
socket gets. The frames are already whole; the variant carries them
and nothing more.

## Compatibility

A reader older than its peer sees `UnknownTag` for the two new
frames, and both `FrameError`s now say so in the words the enqueue
and dequeue frames already used: that is the case the tag exists to
make survivable. The proxy begin frame's enqueue and dequeue tags,
documented one low on each variant, are documented right now.

## The site

`container-proxy-endpoints/agents-begin/response` and
`endpoints/containers-agents-run/response` under `2.3.0` state the
loop — the byte, the chunks, the byte — its two invariants, and the
two new malformed cases.
