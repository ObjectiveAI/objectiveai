# Provider Protocol — Status Report 5

One endpoint is now usable end to end.

Report 4 described two halves that could speak the protocol and six
scopes worth executing. Since then a caller can join a laboratory and do
everything a connector is allowed to do, a plugin can be run and served,
and the protocol lost the one place where a channel meant something
other than one exchange.

## Two corrections to Report 4

**Nothing waits.** Report 4 said "a full queue **waits**", and argued
that a stall is visible where a dropped frame is not. The argument
stands; the premise is gone. Every queue in the caller half is unbounded
now, as the provider half's already were, so no queue is ever full and
nothing this crate does ever blocks on a consumer.

What that trades is a stall for memory. An unread queue grows at
whatever rate the far end sends, and nothing bounds it — so the
obligation on a caller is sharper than a depth would have made it, not
softer: read what you asked for, or drop it. Dropping frees the queue
and makes the router's sends fail, which it ignores.

**The generic response streams are gone.** Report 4 introduced
`ScopeResponseStream` and `ChannelResponseStream` on the reasoning that
the only part of reading a stream that differs is turning one payload
into one item. That turned out to be false in the first two places it
was tried. A watch sends a disconnect when dropped; an MCP exchange
enforces that a head arrives once; a read reports a provider's refusal
and a plugin's scope does not have one. Each endpoint now owns its
stream, and the duplication is smaller than the configuration would have
been.

## A connection is two channels

The largest wire change since Report 4, and it removed a special case
rather than adding a feature.

A Postgres tunnel used to be one channel carrying a socket: successive
request frames on it were successive writes. That made it the only place
in the protocol where a channel took more than one request, and it cost
a close. **Only a responder can finish a channel** — so a provider whose
plugin died without sending pgwire's `Terminate` had no frame with which
to say so, and the caller's database backend stayed open until the scope
ended. The legacy conduit this replaces needed a `Close` travelling both
ways for exactly that reason.

So a connection is now a pair. The provider opens one channel asking the
caller to dial its database and stream back what it says; the caller
opens the other, quoting the same `connection_id`, asking for what the
plugin writes. Each side finishes the channel it answers on, and that
finish **is** the close. No frame type was added.

It is the inversion a write already used, and an image pull before that.
It costs one round trip before the first byte and buys a close in both
directions.

`connection_id` is the provider's to mint and is not a channel number,
for the reason a `write_id` is not: channels are numbered per sender, so
a payload quoting one would be quoting out of a namespace its reader
does not share.

### One request per channel

Which is now stated rather than assumed:

> A channel carries exactly one request. There is never a second request
> frame on a live channel, and the number may not be used again until
> the response stream on it has finished.

It holds for both sides. A rule binding only the client would be a fact
about one implementation, and a reader could no longer take "this
channel" to name one exchange. Postgres was the only violation in the
crate, so stating the rule and fixing that channel were the same change.

## Four proxies

A caller is not only a source of requests. A provider opens channels
back into it for things it cannot reach, and `client` now has a trait
per kind: `McpProxy`, `OciProxy`, `CommandProxy`, `PostgresProxy`. Each
is a trait rather than a callback because the shapes genuinely differ —
one request one answer, one request many answers, and one connection for
a channel's whole life.

What they share is one idea rather than one signature. **Each punts
failure into a vocabulary that already exists**, and each punts to a
different one: an HTTP status, a pgwire `ErrorResponse`, an item in
whatever shape the CLI uses. None has an error variant, because a second
way to say a thing is a second thing to disagree about.

All four answer to `handle`. A type serving both loops and plugins
implements several, and two verbs for one act invited a reader to look
for the difference.

One deliberate exception: `PostgresProxy::handle` takes an MCP plugin's
request frame, and it is the only thing in `client` that names a type
from `endpoints`. A caller decides what a plugin's connection may
*reach*, and it decides that from who is asking and what is running —
neither expressible in a shared type. A trait that could not say what a
security decision rests on would be the worse violation. Nothing in the
specification performs that separation; what the argument does is make
it possible, which it was not when a proxy received only bytes.

## A plugin's scope says nothing

`mcp_plugin::run`'s response used to be `Ready` or an error. `Ready` is
gone, and the response frame is a struct carrying a failure.

It promised a readiness it could not establish. A provider knows when a
CONTAINER has started, which is not the same fact as the MCP server
inside it having bound its port — and this endpoint already documented
that a wrong port surfaces "as an exchange that finishes without an
answer, rather than when the plugin started". The honest test is a call.
A readiness frame was a second, weaker answer to a question already
answered better.

So a plugin that works produces no response frame at all. The scope is
silent until it finishes, and the only thing that ever arrives on it is
a plugin that never came up.

## `execute` has three shapes

Behind the `client` feature, and now for nine of the ten scopes.

Five collapse into a value — an image check and the four volume
operations that end by themselves. `agentic_loop::run` and
`volumes::watch` hand
back a stream. `mcp_plugin::run` hands back a handle, because a plugin's
scope is almost silent and the thing worth holding is the connection to
it: `wait` blocks until the run ends, `error` says what ended it without
blocking, and dropping it sends the stop.

`laboratories::connect` hands back **both**, and it is the first that
does. A connection is two jobs at once and neither is the other's
subject — the scope streams the container's filesystem for as long as
the connection lasts, and a connector reaches into the container on
channels of its own. Folding them together would mean a connector that
stopped reading the filetree had stopped being able to read a file.

`mcp_plugin::run`'s executor is where the three proxies meet. Dispatch
is one task per channel request, uniformly across all three kinds —
which only became possible when a Postgres connection became a pair. A
channel that carried a socket had to find the task already serving it;
a channel that carries one request is answered once and needs no map.

## `connect` is finished

Every ask a connector has: `mcp`, `read`, `write`, `transfer`, and
leaving, which is `Drop`.

Three shapes, because the exchanges are three shapes. A transfer is one
ask and one answer. A read and an MCP exchange are one ask and a stream.
A write is one ask whose **content travels the other way**, on a channel
the provider opens — the only one that needed machinery, and the reason
`execute` spawns a task that holds registered content until the provider
asks for it and routes by write id.

An MCP exchange is the one with no failure of its own. A `502` is a
working stream whose head says `502`, so every error it can report is
this crate's plumbing rather than an answer. Its items are the head or a
piece of body, mirroring the wire, because the two answers MCP gives —
a JSON document and an event stream held open for a session — are the
same sequence and only `Content-Type` says which is arriving.

## Known gaps

- **Nothing has been executed.** No frame has round-tripped. True in
  every report, and now true of nine executors as well as ten scopes.
- **`laboratories::run` has no executor**, which makes it the last large
  one. It is also the only endpoint that must answer an authorization,
  and nothing implements that yet.
- **A provider cannot read what it was asked.** `ScopeHandle` sends
  every frame a provider has and exposes none of what arrives: the
  opening request is held unexposed and channel requests land in a queue
  with no accessor. Unchanged since Report 4, and it is now the largest
  asymmetry between the halves.
- **Auth is discarded in both directions.** Both read loops still drop
  the frame.
- **There is no channel-level cancel.** Three endpoints grew a scope
  stop — a watch's disconnect, a plugin's stop, a connector's disconnect
  — but nothing stops one exchange. A read that is no longer wanted and
  an MCP event stream held open for a session both go on being produced
  until the scope ends. For MCP the remedy is MCP's own `DELETE`; for a
  read there is none.
- **`Sync` on the proxies' boxed streams is probably wrong.** The only
  consumer owns the stream and polls it through `&mut`, so nothing needs
  it — and it turns away `async_stream` generators, which are the
  obvious way to write one. `connect`'s write content does not require
  it and is the proof. Dropping it later widens and breaks nobody.
- **`write_id` uniqueness is unpoliced.** Two outstanding writes sharing
  one sends content to the wrong channel, and only the caller could have
  prevented it.
- **Variant order is wire-significant** in the postcard types.
- **`{ name, digest }` is defined twice** — in `images::check`'s request
  and in `Image`'s two pinned variants.
- **`shared::http::request::Request::body` is a `RawValue`**, so
  tunneled requests carry JSON bodies only.
- **Deliberately unspecified:** what `bytes_used` measures, whether it
  can exceed `bytes`, what an `edit` below it does, and where `memory`
  is enforced given swap is unnamed.
- **Nothing delivers a connection's credentials.** A laboratory id comes
  from a run's own response and an authorization comes from nowhere at
  all — both reach a prospective connector out of band.

Report 4's note that `connect`'s response documented its error as tag
`2` where the constant said `1` is fixed.
