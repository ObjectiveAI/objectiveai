# Provider Protocol — Status Report 7

The crate finished.

Report 6 ended with a provider that could ask for things and read
nothing: three traits waiting to be called, a `ScopeHandle` that sent
every frame and exposed none, no handler behind eight of the eleven
scopes and no dispatch in front of any of them. Since then — fifty-four
commits — the provider learned to read, every scope got its handler,
HTTP left the crate entirely, the way into a container was redesigned
twice and settled, laboratories gained the one piece of cross-scope
state the protocol needed, and a single function now serves a whole
connection. What remains is one word, and it is **authorization**.

This report is longer than its predecessors because the period was. It
is organized by theme rather than by date, except where the order is
the story.

## The provider learned to read

`ScopeHandle` grew two accessors alongside the first handlers:
`request()` handed out the payload that opened the scope, and
`recv_channel_request()` took the channels a client opens inside it off
their queue. Report 6's "largest asymmetry between the halves" closed
with one commit.

Then the handle stopped being exclusive. `send_channel_request` took
`&mut self`, and that single signature was the reason the first
agentic_loop handler had a queue, a `Write` enum and a oneshot — the
MCP task could not open a channel itself, so it had to ask whoever held
the handle. Every method takes `&self` now, the internals that were
exclusive are guarded (the encode buffer, the channel minter; the
socket already was), and a scope is answered from as many tasks as are
serving it, none of which mentions the others. What stays exclusive is
ENDING the scope: `send_response_finish` still consumes the handle, so
one finish per scope is a fact the type enforces rather than a rule the
code obeys, and every handler proves its shares gone with
`Arc::into_inner` before it says so.

And then, at the very end of the period, `request()` died again — for a
better reason than it was born. See the dispatch section: by the time
the crate finished, exactly one thing read a request, and a copy riding
every handle had no reader.

## The handlers arrived in waves

**Seven answer and finish.** The five `volumes` handlers consume
`VolumeManager`; `images::check` consumes **`ImageChecker`**, a fourth
provider trait Report 6 did not have — would you supply this image, to
this caller, where a no is an answer rather than an error; `version`
answers a compile-time constant and is the one handler that asks a
provider for nothing.

**`agentic_loop::run` serves two directions at once.** A task relays
the agent's chunks down the scope; a task serves the agent's tool calls
out on channels; neither is in the other's loop. The ordering that
matters was learned by deadlocking on it: the tool-call server must be
RUNNING before the agent is asked to run, because an agent is free to
want a tool call while it is still deciding how to answer.

Two of its rules outlived every rewrite of its internals. A worker that
panics is reported as an error frame rather than read as a clean
finish, because a caller that saw silence would conclude the loop had
nothing more to say. And **a caller leaving does not end the run**: the
agent finishes, the container is stopped afterwards, and the frames
produced after the departure go nowhere. Tokens were spent and an
upstream was called — a provider that abandons that the moment a
connection drops has given it away, and a caller could take as much as
it wanted and disconnect before anything could be counted.

**`mcp_plugin::run` was written against the byte pipe** — conduits
pre-dialled into the container, a first byte as the taken signal, HTTP
requests reassembled by hand — and its fix history is worth keeping as
evidence: a stale `content-length` forwarded onto a re-encoded body, a
duplicate `Host`, a conduit that span at full CPU when a container
handed back unused pipes, channels ended-not-abandoned in two places.
Every one of those bugs lived in machinery that no longer exists, and
most of them were of one species — a relay RECONSTRUCTING something
instead of carrying it. That species is what the rest of this report
kills.

## The way into a container, three times

**First shape: a byte pipe.** Report 6's `connect`, with `Reader` and
`Writer` associated types. Everything spoke through it and nothing
riding it was one thing, so it went.

**Second shape: HTTP, named.** `serve_http_raw` handed out encoded
requests with a response writer each; `call_http_raw` returned a head
and a body; `call_http_agentic_loop` was the same exchange with the
answer read rather than relayed; `postgres_serve` split off as the one
thing that really is a socket. The HTTP client dependencies — hyper,
hyper-util, http-body-util, eventsource-stream — were pruned, since the
implementation owned the sockets now.

Commands wobbled here, instructively. They were bytes, became an HTTP
exchange — a method, a path, a status, "somewhere to put what is about
the asking" — and were reversed within days: a registry request is HTTP
because a registry SPEAKS HTTP, and nothing speaks a command except the
CLI, which is at the far end of the relay already. The envelope was one
this specification had invented and then had to justify. That reversal
was the first domino.

**Third shape: no HTTP at all.** The rest of this report.

## HTTP left the crate

`shared::http` — request, method, head, response frame — is deleted.
Nothing in the crate names a status, a header, or a verb. The removal
was lossless twice over, for two different reasons.

**MCP is five typed exchanges**, because MCP over Streamable HTTP is
JSON-RPC riding a transport, and the transport was the only part being
carried. `shared::mcp` defines `list_tools`, `list_resources`,
`call_tool`, `read_resource` — one ask, one answer of
`Result | Error(ErrorData)` — and `notifications`, the fifth, whose ask
carries nothing (a bare `GET` with no body is what it mirrors) and
whose answer is a stream of `ServerNotification`s ending cleanly or
with a terminal error. One exchange per channel, because only a
responder can finish one. The four container endpoints'
channel-request enums carry `Mcp`-prefixed variants of these, and the
tunneled `Mcp` variant is gone everywhere.

The relayed `ErrorData` is content, not decoration: `-32601` is "no
such tool" and `-32602` is "the arguments were wrong", and a caller
told only that something failed can act on neither.

**OCI is verbatim bytes**, because the distribution protocol has
nothing underneath it — its semantics ARE HTTP's. `shared::oci` is a
request that fits one frame (a pull has no request body; `GET` and
`HEAD` only, `Range` is a header) and a response frame that is a bare
byte slice: no head, no error variant, no ordering rule. A registry
that refuses already knows how to say so — a `404`, a `429` with a
`Retry-After` — and a caller that cannot reach its registry is a proxy
that cannot reach its upstream, which HTTP also has a way to say. Both
`Encode` and `Decode` are `Infallible`, and the channel's finish is the
answer's end.

The insight underneath both, learned from how nginx survives: header
bugs come from RECONSTRUCTION, not carriage. A relay that hands over a
parsed head has read a status and must then decide what to do about
headers that describe the message rather than the answer — and every
such decision in this crate's history was a bug. `Range` resumes and
`HEAD` probes work now not because anything was taught about them but
because nothing can get them wrong.

**What radiated outward.** `McpProxy` is five methods taking rmcp
params directly; `OciProxy` is one method, bytes in, an infallible
stream of bytes out. Four executors' handles grew the five MCP methods
and a named `McpNotificationsStream`, with an `McpError` whose only
container-speaking variant is `Mcp(ErrorData)` — everything else is
this end failing to ask or to read. The server's `OciStream` collapsed
to `Item = Bytes` with three errors where six stood, its `headed` flag
and ordering rule deleted with the head they policed.
`ClientRegistry` takes an `oci::request::Request` and borrows
`&ScopeHandle` plain. And a wire convention got named on both halves at
once: **a channel that finishes with nothing before it is a provider
that could not serve the exchange at all** — the caller's error
vocabulary calls it `Unanswered`, and a wrong port is documented as
looking exactly like that.

`Infallible` spread as a consequence. `mcp_plugin`'s server
channel-request frame encodes infallibly — a registry request is bytes
copied, a connection id four known bytes, a command bytes — and the
crate's idiom for discharging one, an empty match, is now
compiler-checked at every site: an empty match only typechecks on an
uninhabited type.

## `Container`, final shape

Eleven associated types, twelve methods, and HTTP named nowhere — a
container really is an HTTP server, and the trait's position is that
this is the implementation's whole business, an implementation being an
rmcp client on one port and an rmcp server on another.

- **`mcp_serve`** replaces `serve_http_raw`: a stream of
  `(McpRequest, McpResponder)` pairs. The request enum has five
  variants and not more — `initialize`, `ping`, `subscribe` never cross
  this wire, and there is no variant for them to arrive as. The
  responder has a method per variant: four consume `self`, because an
  exchange answered once cannot be answered again and taking `self` is
  how that stops being a rule and becomes a fact; `notification` takes
  `&mut self` for as long as there is anything to say, and
  `notifications_finish(Option<ErrorData>)` ends it — the option
  because the far server can end that stream by refusing, and a
  container told only "no more" could not tell a server that finished
  from one that broke.
- **Five inward `mcp_*` methods** mirror `McpProxy`, with a nested
  `Result`: the outer `Err` is this crate not reaching the container,
  the inner one is the container's own MCP server refusing in its own
  vocabulary. Flattening them would make "the port is wrong" and "no
  such tool" the same answer, and only one of those is about the
  plugin.
- **`agentic_loop`** takes the caller's body as the `&RawValue` it
  arrived as — a field this crate does not model survives the trip —
  and returns a stream of parsed chunks. SSE reassembly moved into the
  implementation, which is the only thing that read the head; the
  handler's hand-rolled `Chunks` buffer died.
- **`postgres_serve`** yields a `(reader, writer)` pair per connection
  the plugin opens. The item's existence is the fact, so the pre-dial
  loop and the first-byte signal are gone.
- **`command_serve`** yields one whole ask per command beside a writer
  its answers go into; dropping the writer closes the pipe, which is
  the plugin's entire vocabulary for "the command is over". The
  container-side protocol lost its four-byte exchange id: the old
  handler split it off and threw it away, nobody ever read it, and one
  pipe being one command means the pipe is the correlation.
- **`filetree`** is a fresh subscription per call — one snapshot, then
  deltas — because a connector arriving an hour into a run needs the
  whole tree before any delta means anything, and the runner's stream
  is an hour past its snapshot. Items are infallible; a watch that
  breaks ends its stream and nothing else, a broken watch not being a
  broken laboratory.
- **`read`, `write`, `stop`** are as Report 6 left them.

A naming sweep made every stream say so: `McpRequestStream`,
`McpNotificationsStream` (plural, matching the wire variant, and the
client half renamed to agree, filename included),
`AgenticLoopStream`, `PostgresConnectionStream`, `FiletreeStream`,
`CommandStream`.

**The plugin handler came back on this trait**, much smaller than it
left: typed dispatch with owned params (the clone-the-bytes-and-decode-
again dance is gone), one `answer` helper enforcing at-most-one-frame-
then-finish on every path, a bare finish where the container is
unreachable — which is the documented wrong-port shape, not an
invention. `relay`, the header filter, `accept`, and the hyper
request-building are deleted outright.

## The laboratories handlers, and the registry

The last two handlers had never existed in any form, and they need what
no other endpoint does: **two scopes meet at one container**. A
laboratory is created on one scope and reached from others — a
`connect` names it by id from a different scope, possibly a different
connection and a different caller entirely, and a `transfer` names
another laboratory's container as its destination. Scopes are
strangers; something above all of them has to resolve an id.

**`server::laboratories::Laboratories`** is that thing: the one piece
of cross-scope state on the server half, a concrete type like
`ClientRegistry` because resolving an id is this crate's bookkeeping
and not something a provider could implement. The run handler inserts
on deploy and removes on teardown; connects and transfers look up. A
provider makes ONE and hands it to every handler on every connection —
two registries would be two worlds that cannot see each other's
laboratories.

**The id is a capability.** Minted as a random UUID (the crate's one
new dependency, `uuid` with `v4`, server-side only), because holding
the id is what entitles a caller to NAME the laboratory: a transfer is
authorized by knowing the id and by nothing else, so an id anyone could
predict is a container anyone could reach. A connect is the exception —
the id only starts the conversation, and whether the connector may
attach is the runner's answer.

**An entry holds the container and the run handler's ear, and
deliberately not its `ScopeHandle`** — the finish proof requires that
nothing else ever holds a share. Everything a connector needs said on
the run's scope travels as an event: `Authorize`, carrying the
connector's address and opaque credential with a oneshot for the
verdict and nickname, and `Disconnected`, owed for every arrival the
runner granted — the graceful leave, the vanish, and the laboratory
ending all send it, because the runner saw every arrival and
"nothing can disconnect that was not authorized first" is half of an
invariant the connect handler keeps the other half of.

The event sender's second job is the elegant one: the run handler owns
the receiver, so however the run ends, the receiver drops and every
connector's `closed()` resolves. That is the whole mechanism behind
"a connection cannot outlive the thing it joined" — the run never
learns who joined, and they all find out it is over.

Two provider-reserved environment names carry what the request's
`name` and `initial_cwd` exist for — `DIVERGE_LABORATORY_NAME`
verbatim and `DIVERGE_LABORATORY_INITIAL_CWD` as a JSON array of
components, over the caller's environment so they win collisions. The
laboratory's MCP port is a constant in the registry's file because both
handlers must agree on it, and two copies would be two numbers the day
one changed.

A write's content channel becomes the stream `Container::write`
consumes through an adapter that maps the wire's cases onto
`ContentError::Wire`; a transfer maps the source's read failures onto
`ContentError::Container` — the two-vocabulary split Report 6 designed
finally has both callers.

## The dispatch in front of everything

`server::handle` is the function Report 6's gap list was pointing at
without naming. One call per connection:

```rust
pub async fn handle<D, V, I>(
    session: Session,
    client_identity: String,
    address: IpAddr,
    deployer: Arc<D>,
    volume_manager: Arc<V>,
    image_checker: Arc<I>,
    laboratories: Arc<Laboratories<D::Container>>,
)
```

It reads what the session yields, routes by `ClientRequest` — the one
place the tag values meet, so no second tag table exists to drift —
and spawns the endpoint's handler. Spawning is not a preference:
polling the `Session` is what runs the connection, so work done inline
starves every scope of the frames that would feed it, including its
own.

**The decoded request travels into the handler.** All eleven endpoint
handles take their `request::Frame` as a parameter now; none decodes
its own, and their decode-failure arms are deleted. A request nobody
can read is `Invalid`, answered by finishing the scope over nothing —
eleven endpoints have eleven error vocabularies and an invalid request
names none of them, and a bare finish is already what the wire means by
a request that could not be served.

**Which is why `ScopeHandle::request()` died.** A request is read
exactly once, by the dispatcher; a copy riding every handle had no
reader. The `Session` now yields `(Bytes, ScopeHandle)` — the payload
beside the thing that answers it — which also dissolved the
dispatcher's borrow problem (nothing borrows the scope while it moves
into its task) and turned agentic_loop's raw body from a copy into a
refcounted slice, delivered as a parameter.

**The ending drains and never aborts.** When the session yields `None`
the connection is gone; dropping the session severs every scope's
feeds, and the handlers wind down through their own teardown —
containers stopped, registry entries removed, finishes written into a
socket that is no longer listening, harmlessly. Aborting instead would
tear through all of that and end runs a caller had already paid for.

`client_identity` and `address` are parameters because auth is still
the session's documented gap: the connection's identity is whatever the
provider established at the upgrade, and this crate cannot yet see it.

## Report 6's gaps, item by item

- **"A provider cannot read what it was asked"** — closed, then the
  accessor itself was superseded by the pair the session yields.
- **"Nothing implements or calls any provider trait"** — every trait
  is consumed by handlers, and the handlers by the dispatcher. Nothing
  IMPLEMENTS them, which is the next crate's job, not this one's.
- **"This crate has no HTTP codec, and `connect` needs one"** — the
  decision Report 6 deferred was dissolved rather than made. `connect`
  is gone, the tunnel is gone, and no codec is needed by anyone:
  hand-write, take the dependency, or push onto the provider turned out
  to have a fourth answer, which was to delete the question.
- **"Auth is discarded in both directions"** — still true, and now the
  only structural gap. See below.
- **"There is no channel-level cancel"** — still true. A notification
  stream a caller abandoned is produced into the void until the scope
  ends; dropping the client-side stream frees the caller and tells the
  provider nothing.
- **"`write_id` uniqueness is unpoliced"** — still true, and still
  only the sender's to police. A Postgres `connection_id` is no longer
  in this list: the provider mints it now, counting up on one task.
- **"Variant order is wire-significant in the postcard types"** —
  still true.
- **"`{ name, digest }` is defined twice"** — still true.
- **"Tunneled requests carry JSON bodies only"** — dissolved with the
  tunnel.
- **"Deliberately unspecified" volume semantics** — unchanged, and
  still deliberate.
- **"Nothing delivers a connection's credentials"** — unchanged: a
  laboratory id and a connect authorization both reach a connector out
  of band, and the protocol says nothing about how.
- **"A wrong port is not detectable at deploy time"** — unchanged in
  substance, restated in vocabulary: a wrong `mcp_port` is now an
  exchange that finishes without an ANSWER, there being no heads left
  to finish without.

## Known gaps

- **Authorization is the gap.** An `Auth` frame is read and discarded;
  a provider on an outgoing connection cannot send the one it owes; the
  dispatcher takes `client_identity` and `address` as the documented
  stopgap. This is the one piece of the protocol that is designed
  around rather than designed, and it is the next piece of wire work.
- **The hard-coded numbers.** Four agent image names are `"TODO"`,
  their memory and disk limits are unjustified, agentic_loop's two
  ports and the laboratory MCP port are constants with `TODO`s — all
  settled when the images and the laboratory server exist, which are
  deliverables outside this crate.
- **`Invalid` flattened the parse diagnostics.** The per-endpoint
  "your request did not parse" answers went with the handlers' decode
  arms; a malformed request now reads as unanswered. A deliberate
  trade — single source of truth for the tags and no double decode —
  and recorded as one.
- **Handlers trust the dispatcher.** They are public and take a
  decoded frame on faith; calling one with a frame that does not match
  the scope's request is possible and unchecked. The contract is
  documented at every signature.
- **Nothing has been executed** — true in every report, and now true
  by construction rather than by omission. The crate creates no
  runtime, dials no socket, and implements none of its own traits; a
  provider is `Session::new` and one `handle` call per connection plus
  four trait implementations, a caller is the executors, and the first
  pair of them is what exercises this. This crate is the specification
  those two are checked against, and it is done being written.
