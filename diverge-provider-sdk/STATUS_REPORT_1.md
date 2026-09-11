# Provider Protocol — Status Report 1

The provider protocol is defined by this crate. Not described by it —
defined. Prose states requirements; these types state the messages,
and where the two disagree the crate is correct and the prose is a
bug. That is the same relationship MCP's `schema.ts` has to its
specification, and it exists for the same reason: two documents
describing one wire format eventually stop agreeing.

What follows is where that definition stands.

## The wire

Every message is one WebSocket binary frame with a fixed nine-byte
header:

```text
[type: u8][scope: u32 big-endian][channel: u32 big-endian][payload…]
```

Fixed rather than varint-encoded. Varints would have saved about four
bytes on a typical frame — under two percent of a chunk carrying JSON
— and charged for it with a payload offset that is a parse result
instead of a constant. There is no length prefix, because WebSocket
already delimits messages and carrying one would pay twice for the
same fact.

A **scope** is one client request and everything that follows from it.
The client does not choose it: it sends a request with no scope, and
the server mints one in its acknowledgement. A **channel** is one
exchange inside a scope. Channel `0` is the answer to the client's own
request; any other channel was opened by one side to ask the other for
something.

Channels are numbered **per sender**. A client's channel `5` and a
server's channel `5` are different channels, told apart by which
direction a frame travelled. Neither side gives up half the space and
neither has to negotiate.

### Frame types

| type | client | server |
|------|--------|--------|
| 0 | auth | auth |
| 1 | request | — |
| 2 | — | response ack |
| 3 | — | response |
| 4 | — | response finish |
| 5 | channel request | channel request |
| 6 | channel response ack | channel response ack |
| 7 | channel response | channel response |
| 8 | channel response finish | channel response finish |

Auth leads because it leads in time — nothing may precede it, and a
peer reading a connection's first byte should not have to look past
the reply types to learn whether it is one. After that the order is
the order things happen in: a request opens something, and the ack,
responses and finish that answer it follow immediately behind. The
same four beats twice, once per level.

A number means one thing in both directions, and the blanks are what
keep it that way. Only a client opens a scope, so only a client sends
`1`; only a server answers one, so only a server sends `2` through
`4`. Closing those gaps would save two byte values and cost the
property that a type identifies a frame without reference to who sent
it.

**Auth has no reply.** A credential that is accepted is followed by
the connection working; one that is not is followed by a close. A peer
that has not authenticated cannot make the far end compose anything,
so a bad credential earns no bytes to amplify and no answer to read a
reason out of. Wrong, expired and unknown are indistinguishable from
outside, which is the point. What it costs is diagnosis, and the docs
say so.

**The frame layer parses nothing.** Every payload is bytes in both
directions. This layer splits a header and names a frame kind; what a
payload means belongs to the protocol carrying it, and is
discriminated by a tag inside the payload rather than by anything out
here. That is why the type space is closed — growth is a new tag
value, where the reader that cares is already looking.

## Payloads

Two traits, deliberately narrower than serde's. A `Deserialize` impl
says how a type maps onto *some* format and leaves the choice to the
caller; `Encode` and `Decode` say *which* format, once, in the type.

```rust
trait Encode { type Error; fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error>; }
trait Decode<'a>: Sized { type Error; fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error>; }
```

`Writer` is append-only by construction: the header is already in the
buffer, and nothing on its public surface can reach a byte that was
already there.

There is no single serialization. MCP and registry traffic is JSON,
because those channels relay documents that must survive
byte-identical. Filetree and filesystem traffic is **postcard** —
field names dropped entirely, every length and integer varint-encoded
— because it is high-volume and relays nothing. A connection request
is neither, because it carries opaque bytes that JSON can only hold as
base64 and that serde would silently mis-encode.

## Scopes

Six requests open a scope. Each is one tag value at the front of a
`ClientFrame::Request` payload.

**`agentic_loop` (0)** — run an agent and stream what it does. The
request is an `Agent`, a prompt of MCP content blocks, and an optional
continuation. Four upstreams: OpenRouter, the Claude Agent SDK, the
Codex SDK, and Python. Everything is post-transform — a system prompt,
personality and prefix messages shape a request before a provider sees
it, so a provider never rewrites what it was given. The answer is a
stream of chunks: reasoning, text, image and audio content, tool
calls, refusals, tool responses, usage, errors, and the continuation.
Along the way the provider opens channels back at the client for
**MCP** and **Postgres**, because the agent runs beside the provider
while the tools and the database live with the client.

**`images::check` (1)** — can you supply this image. A repository name
and a manifest digest, with no registry: where a provider gets an
image is the provider's business, and a caller naming a registry it
cannot reach is asserting something it has no standing to assert. The
answer is available or unavailable, and unavailable says nothing about
why — distinguishing "I do not have it" from "I will not serve it to
you" would tell an unauthorized caller that a private image exists.

**`filesystem::list` (2)** — which directories may I watch. No
parameters, so the request is a single tag byte. The answer is
directories, each a name and a path, where the name is a **label the
provider chose** and is also the handle.

**`filesystem::watch` (3)** — watch one of them, by name. Naming
rather than pathing is the whole access model: a caller cannot watch
what it was not offered, cannot escape upward, and cannot probe by
reading an error, because a path it invents is not something the
request can express. The answer is a filetree stream.

**`laboratories::create` (4)** — run an image. Carries the image type
and reference, a memory ceiling in bytes, environment, mounts, and the
directory agents land in. A mount names an offered directory and an
offset within it, so the same access model holds. The scope is the
container's **life**.

**`laboratories::connect` (5)** — join a container somebody else made.
A container id and an opaque authorization.

## Filetree

One shape, used by three things. A snapshot establishes the tree; every
later frame names one node and says what became of it — inserted,
modified, moved, or removed. Nodes are files, directories or symlinks;
a symlink is the link itself and is never followed, so a dangling one
is a leaf rather than an error.

Deltas carry **complete** values, never patches against state the
consumer is assumed to hold, which is what makes replay harmless and
therefore at-least-once delivery safe. `Root::update` is the canonical
fold, so two consumers fed one stream cannot disagree about what it
meant.

## Laboratories

A creation is the most complete exchange in the protocol, and the only
one with channels going **both ways at once**.

The provider opens channels to pull the image when the caller is the
one serving it. It does this by standing up a registry endpoint,
pointing its own container runtime at it, and relaying — the runtime
pulls normally, asks only for blobs it does not already hold, and its
cache is therefore the only cache. `Range` resumes and `HEAD` probes
work because nothing had to be taught about them. The scope rides in
the repository name, so one endpoint serves every concurrent creation
and each request routes itself.

It also opens an **authorize** channel each time a connector arrives.
The payload is that connector's authorization, relayed verbatim from
its `connect` request. The provider does not read it and could not
usefully — what makes one connector acceptable is something only the
creator knows, which is exactly why the creator is who gets asked. The
answer is one byte.

The caller opens channels the other way, to reach the **MCP server
inside** the container — the mirror of the agentic loop, where the
container is the thing a caller cannot dial.

Channel 0 carries the container's id, its connector count, and its
filesystem, in no particular order. A container can be running and
reporting before its provider has finished deciding what to call it,
and a connector can arrive at any moment. Nothing waits on anything.
The count is a **value**, re-sent whenever it changes, so a lost or
replayed frame leaves a reader with a number rather than a drift.

A connection gets the same filesystem and the same count, opens the
same MCP channels, and gets no id — it supplied one to get here.

## What is not here

No transport, no client, no server. A provider binds these types to a
WebSocket; the frameworks that make that easy are separate crates.
Keeping this one free of runtime concerns is what lets both sides of
the protocol depend on it.

No ordering rules, either, beyond what a tag can express — those are
properties of a *sequence*, and a schema constrains a document rather
than a trajectory. They belong in prose with RFC 2119 keywords and in
the frameworks, so an author cannot violate a requirement that only a
document states.

## Known gaps

- **Nothing has been executed.** Every type compiles and every doc
  link resolves; no frame has round-tripped. The decoders are correct
  by inspection, which is not the same thing.
- **Tag values are allocated across modules that do not know about
  each other.** Six scope tags live in six files, each documenting the
  other five by hand. Nothing checks.
- **Variant order is wire-significant** in the postcard types, since
  external tagging writes an index rather than a name. New variants go
  on the end; nothing enforces it.
- **`shared::http::request::Request::body` is a `RawValue`**, so tunneled
  requests carry JSON bodies only. Both current users are fine — a
  registry `GET` has no body — and it becomes wrong the day something
  needs to `PUT`.
