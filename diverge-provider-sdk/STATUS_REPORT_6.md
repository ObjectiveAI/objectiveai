# Provider Protocol — Status Report 6

The provider half started asking for things.

Report 5 described ten scopes, nine of which a caller could execute, and
a server that could speak the protocol without being able to act on it.
Since then the eleventh scope arrived, the last executor was written, the
wire lost five bytes it was carrying out of habit, and — the change that
takes up most of this report — the provider half grew a set of traits
naming the parts this crate cannot know: where a container runs, how to
reach one, and whose directories are whose.

## One correction to Report 5

**Report 5 omitted the naming sweep.** Before it was written, every
channel end in the crate had been renamed so that the suffix says which
end it is: `_sender` or `_receiver`, with a singular noun in front.
`Channel`'s field was `responses` and is `response_receiver`, on both
halves; the router's ends were `registrations` and `closed`; a session's
were `notices`, `inbox` and `finished`; and the three executors written
just before it had added `requests`, `writes` and `plugin_writes` on top
of the drift. `PostgresProxy::handle`'s argument became
`request_receiver`.

That is public API on two types and it touched more files than anything
else in that period. Report 5 described the executors that shipped
alongside it and did not mention it at all. It is not a change since
Report 5 — it is a change Report 5 should have contained.

The report itself was restored, separately, to exactly what it said
after two edits were made to it. Reports are not amended.

## The wire moved in six places

Everything here changes bytes. Nothing else in this report does.

**An eleventh scope: [`version`], tag `10`.** A caller asks what
protocol a provider speaks, and the provider answers with a string and
finishes. The request carries nothing — it is a unit struct and one tag
byte, the same shape `volumes::list` has — and the response is a bare
`&str` with no framing of its own.

It is the only endpoint whose response is not an enum and not a struct
with fields. There was nothing to discriminate and nothing to name, so
it carries the string and stops.

**Five tag bytes are gone.** A tag exists to tell variants apart, and
five of them were in front of payloads with nothing to tell apart:

| where | what it was |
|-------|-------------|
| `mcp_plugin::run`'s response | left over from when `Ready` sat beside the error |
| `version`'s response | never had a second case |
| `agentic_loop::run`'s server channel request | one variant |
| `laboratories::connect`'s server channel request | one variant |
| `volumes::watch`'s client channel request | one variant |

The first is the instructive one. That response used to be an enum, the
tag told `Ready` from an error, and when `Ready` was removed — Report 5
covers why — the byte stayed. It was kept out of forgetfulness rather
than intent, and reading it as a convention would have been reading a
leftover as a rule.

They are not held open against a second case arriving later. A tag spent
on a choice nobody is making is a byte on every frame and a case in
every reader, paid now against something that may never happen. Five
`FrameError` enums lost their `UnknownTag` and `Empty` variants with
them.

The scope-opening requests keep theirs, because eleven of them share one
`ClientFrame::Request` and the leading byte is the only thing that says
which.

**A plugin declares three ports, not one.** `mcp_plugin::run`'s request
had `port`; it now has `mcp_port`, `postgres_port` and `command_port`.
They are ports INSIDE the container — a provider dials in to them, and
what it reaches them as on its own side is its business and no caller
sees it.

**Two of the three are optional.** `mcp_port` is required, because a
plugin that serves no tools is not a plugin. The other two are conduits:
`None` means the provider connects to nothing, opens no channel, and the
caller is never asked to dial one. Declared rather than discovered,
because a plugin that is slow to bind and a plugin that never will look
identical from outside and only one of them is worth retrying.

The consequence reaches the Postgres channel pair Report 5 introduced. A
plugin that named no `postgres_port` never has one of those channels at
all — so opting out costs nothing rather than costing an idle tunnel,
and so does opting in and staying quiet.

**A laboratory's authorize nickname is no longer optional.** The
`Authorize` channel response carried an optional string beside its
decision; it now carries one always, in the `Authorized` variant. A
presence byte and two `FrameError` variants went with it. A decision to
admit somebody without saying what to call them was a state nothing
downstream had a use for.

## Every scope has an executor

The eleventh and last is **`laboratories::run`**, which Report 5 named as
the largest remaining gap. It returns the same pair `connect` does — a
stream and a handle — and it is the only executor that takes two
provider-supplied traits: an `OciProxy`, for an image the caller serves,
and a `LaboratoryConnectionAuthorizer`.

```rust
pub async fn execute<O, A>(
    handle: &Handle,
    request: &request::Frame,
    oci_proxy: Arc<O>,
    authorizer: Arc<A>,
) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError>
```

Its stream yields a `RunFrame` — an `Id`, a `Filetree` change, or a
`Disconnected` — and its handle carries `stop`, `mcp`, `read`,
`transfer` and `write`, which is `connect`'s surface plus the ability to
end the laboratory rather than merely leave it.

`version`'s executor is the other new one, and is a single call
returning `Result<String, ExecuteError>`.

So the count Report 5 gave as nine-of-ten is now eleven of eleven.

## `execute` is a folder, and a public module

It was a file in nine endpoints, and one of them had outgrown it. Now
every endpoint has `client/execute/` with `execute.rs` beside
`execute_handle.rs`, `execute_stream.rs` and whatever else that endpoint
needs — `laboratories::run` has six files in there.

It is also `pub mod execute` rather than a re-export, so the public path
is `endpoints::…::client::execute::execute` and the types beside it are
addressable. The module is behind the `client` feature.

Thirty-five files moved and no bytes changed.

## Teardown is a method, not a destructor

Three executors ended their scope from `Drop`: a watch sent its
disconnect, a plugin its stop, a connector its disconnect. All three now
have an async method instead — `stop` on `mcp_plugin::run` and
`laboratories::run`, `disconnect` on `laboratories::connect` and
`volumes::watch` — and no destructor sends a frame.

The reason is that a destructor cannot await. What it can do is spawn,
or block, or fire and hope, and all three were doing a version of the
third: the frame was queued and the caller had no way to learn whether
it left. A method returns `Result` and the caller knows.

`volumes::watch` changed shape to accommodate it, and now returns
`(ExecuteStream, ExecuteHandle)` like the other two — there was
previously nothing to hang the method on. Prebuilt teardown frames and
the `serving` join handles that existed to survive `Drop` are gone, and
`ExecuteError` lost the `Disconnect` and `Stop` variants that reported a
teardown failing during setup.

## The caller half's traits

Five now, and the fifth is the first that is not a proxy.

**`LaboratoryConnectionAuthorizer`** is asked whether a connector may
join, and answers `Decision::Denied` or `Decision::Authorized(String)` —
the nickname discussed above. It sits beside `McpProxy`, `OciProxy`,
`CommandProxy` and `PostgresProxy` in `client/`, and it is the only one
that decides something rather than relaying it.

**The proxies' boxed streams no longer require `Sync`.** Report 5 listed
this as probably wrong and it was: the only consumer owns the stream and
polls it through `&mut`, so nothing needed the bound, and it turned away
`async_stream` generators — which are the obvious way to write one.
`Container::write`'s content stream never required it and was the proof.

## The provider half now asks for things

This is the largest change since Report 5 and none of it is on the wire.
Everything before this was a message; `server/` had two types that did
something with one. It now has three traits a provider implements and
five concrete types that go with them, and nothing implements any of it
yet.

### `ContainerDeployer`

Three methods, one per image source — `client`, `server`, `registry` —
each taking a `&Deployment` and returning an associated `Container`.
`client` additionally takes a `ClientRegistry`, because pulling an image
the CALLER serves means opening a channel on a scope, which is this
crate's machinery rather than a provider's.

It is generic across the three endpoints that put a container somewhere,
because they differ in what goes IN one and not in how one is deployed.
A deployer that took a request frame would take three of them and be
three deployers.

Two decisions worth keeping:

**Its error is an associated type**, not `shared::error::Error`. A
provider has failures of its own shape and the handler is what turns one
into a frame. A crate that named a provider's error type would be
describing something it cannot see. The same holds for `Container` and
`VolumeManager`.

**A container is RUNNING when a method returns.** There is no
create-then-start, because pre-start file injection is a Podman-shaped
lifecycle — `create`, then `cp` a tar, then `start` — and a runtime built
on images rather than containers cannot offer it. What the trait does
NOT promise is what is running INSIDE: a container that started is not a
server that bound its port, and the honest test of the second is a call.

### `Container`

What a provider holds a running container by, with four methods, and the
line between what belongs here and what does not was restated once
during this period. It is not "what a provider cannot do from outside" —
`connect` disproved that. It is **what needs something only the deployer
learned**.

- **`stop`** — infallible. `impl Future<Output = ()>`, no `Result`. A
  container that will not stop is not a fact a caller can act on, and a
  provider that wants to know has its own logs.
- **`read`** — a path, and a boxed stream of `Result<Bytes, Error>`.
- **`write`** — a path and a boxed stream in, a `Result` out.
- **`connect`** — a port, and `Result<(Reader, Writer), Error>` where
  both are associated types bounded `Send + Unpin + 'static`.

`connect` is a byte pipe rather than an HTTP client, and that was the
decision in it. Everything reaching a container port is a byte protocol
underneath — MCP is HTTP, a conduit is pgwire, a command relay is
neither — so an HTTP-shaped method would have served one of them and
been worked around by the rest. `Unpin` is on both halves because
`StreamExt::next` and `SinkExt::send` require it: an implementation
holding something `!Unpin` boxes it once at construction, where a
consumer without the bound pays at every call.

It returns a `Result` rather than a stream that begins by failing,
because nothing listening on that port is the ordinary case and folding
it in would make "could not connect" indistinguishable from "connected,
then closed".

**`ContentError<E>`** exists so a `write` can take content from either
place. A transfer between two containers reads one container and writes
another; a write from the wire reads a caller's frames. The stream's
items are `Result<Bytes, ContentError<E>>` with `Wire` and `Container`
variants, so a failure says which side stopped and a handler can tell a
caller's problem from a provider's. It lives in `container.rs` rather
than a file of its own, being a companion type to that trait and nothing
else's.

### `VolumeManager`

The newest, and the one that covers the five `volumes` endpoints:
`list`, `create`, `edit`, `delete`, `watch`. One trait rather than five,
because they share the state and not merely the subject — what `list`
reports is what `create` added, and five traits would be five views of
one map with nothing saying they had to be the same map.

Every method takes a `client_identity`, because a volume's name is
unique within the caller it was listed to and not globally. It is an
argument rather than a constructor parameter so that one manager serves
every caller, rather than being built and dropped around a namespace
that outlives the connection.

`watch` returns `Result<Stream, Error>` rather than folding its failure
in, diverging from `Container::read` deliberately: a path inside
somebody else's container may not be checkable without beginning to read
it, and a name in a namespace this trait owns always is. Both flatten to
the same `Error` frame, and that flattening is the handler's.

There is no `unwatch`. Dropping the stream ends the watch, which is one
fewer method and no way for a handler to forget half a pair.

It declines to decide three things, and says so: whether `create` over a
taken name fails, whether `edit` below `bytes_used` fails, and what
`delete` does to a volume something is still using. The wire has one
error per endpoint and no vocabulary for the reasons.

### The types that go with them

**`Deployment`** is what every container asks for once the differences
between the three endpoints are set aside: `memory`, `disk`,
`environment`, `mounts`, `ports`. The image is not in it — it is an
argument to whichever method is called, because the METHOD is the
source, and carrying an `Image` here as well would let a caller hand
`Client` to the method that pulls from a registry.

**`ports` is declared, not asked for later**, and rootless Podman is the
case that settles it. A rootless container has no address the host can
route to, so a published port is the only way traffic gets in — and
publishing happens at create. A protocol that let a port be named at
request time would work rootful and be quietly broken rootless, which is
the worst way to be wrong.

**`server::Mount`** is the wire `Mount` plus `client_identity`, first
field. A `host_name` is a `Volume::name` and a name resolves in one
caller's namespace, so a bare name answers nothing. A caller could not
have supplied the other half — the only owner it can name is itself, and
a field a caller could state is a claim to check rather than a fact to
know. Per-mount rather than per-deployment, because the name it
qualifies is on the mount and one owner for all of them would rule out a
container drawing on more than one caller's volumes.

It is not the plugin's `Identity`, which is a different question with a
similar name: that one is what a plugin is TOLD about its caller. A
laboratory has none and still takes mounts.

The wire `Mount` moved to `shared::container::request` in the same
period, since it is no longer a laboratory's alone.

**`ClientRegistry`** is concrete rather than a trait, because pulling an
image the caller serves is this crate's machinery. It borrows a
`&mut ScopeHandle` and takes an inline `fn` wrapper — a `Wrap` type alias
existed briefly and was written out, the crate having no aliases except
in the frame layer.

**`OciStream`** is what comes back from one. It owns the whole `Channel`
rather than the receiver out of it, because a server `Channel` notifies
its session on drop and moving a field out would have skipped that. It
yields `OciFrame::{Head, Body}`.

### The two traits do not know about each other

A `Mount` reaches a `ContainerDeployer` as a name and an identity, and
turning that into a directory is the deployer's business — the same way
publishing a port already is. So `VolumeManager` has no `resolve`, and
`ContainerDeployer` has no opinion about volumes. A provider implements
both and knows its own layout; a crate that put a path between them
would be inventing a representation for a directory that neither needs
to agree on.

## Known gaps

- **Nothing has been executed.** No frame has round-tripped. True in
  every report, and now true of eleven executors, eleven scopes, and
  three provider traits.
- **A provider cannot read what it was asked.** `ScopeHandle` sends
  every frame a provider has and exposes none of what arrives: the
  opening request is held unexposed and channel requests land in a queue
  with no accessor. Unchanged since Report 4, and still the largest
  asymmetry between the halves — now larger, because there are traits
  waiting to be dispatched to and nothing to dispatch from.
- **Nothing implements or calls any provider trait.** What would is the
  handler layer, which is still not written, and which was deliberately
  deleted rather than carried once the client half found its shape.
- **This crate has no HTTP codec, and `connect` needs one.**
  `shared::http::request::Request` is structured — a method, a path,
  headers, a `RawValue` body — not wire bytes. Relaying MCP over a byte
  pipe means turning one into an HTTP/1.1 request and parsing the answer
  back. `axum` is pulled with `ws` and without `http1` or `http2`
  precisely so nothing here can speak HTTP, so this is a decision
  deferred rather than made: hand-write a small codec, take the
  dependency, or push the job onto the provider. It is created by
  `connect` and named here rather than left to be discovered.
- **Auth is discarded in both directions.** Both read loops still drop
  the frame.
- **There is no channel-level cancel.** Four endpoints now have a scope
  stop; nothing stops one exchange. A read that is no longer wanted and
  an MCP event stream held open for a session both go on being produced
  until the scope ends.
- **`write_id` uniqueness is unpoliced**, as is a Postgres
  `connection_id`. Two outstanding writes sharing one sends content to
  the wrong channel, and only the sender could have prevented it.
- **Variant order is wire-significant** in the postcard types.
- **`{ name, digest }` is defined twice** — in `images::check`'s request
  and in `Image`'s two pinned variants.
- **`shared::http::request::Request::body` is a `RawValue`**, so
  tunneled requests carry JSON bodies only.
- **Deliberately unspecified:** what `bytes_used` measures, whether it
  can exceed `bytes`, what an `edit` below it does, where `memory` is
  enforced given swap is unnamed, and — new with this report — what a
  `delete` does to a volume that is mounted or watched.
- **Nothing delivers a connection's credentials.** A laboratory id comes
  from a run's own response and an authorization comes from nowhere at
  all — both reach a prospective connector out of band. The authorizer
  trait decides on one; it does not explain how the connector got it.
- **A wrong port is not detectable at deploy time.** The container
  starts fine and nothing answers: a wrong `mcp_port` is an exchange
  that finishes without a head, a wrong `postgres_port` or `command_port`
  is a conduit that never carries anything. An ABSENT one is a different
  thing and is not a mistake.

Report 5's two open items are closed: `laboratories::run` has its
executor, and `Sync` is off the proxies' streams.
