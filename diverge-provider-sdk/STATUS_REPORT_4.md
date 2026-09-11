# Provider Protocol — Status Report 4

The crate stopped being types only.

Reports 1 through 3 described messages. Since Report 3 it has grown two
halves that speak them — a caller's and a provider's, each behind a
feature, each off unless asked for. The specification did not change
much. What changed is that there is now something to point at when
asking whether it works.

## Two corrections to Report 3

**A write is no longer correlated by a channel.** Report 3 explained at
length why the content of a write travels as responses on a channel the
provider opens, and that reasoning still holds — only a responder ends a
channel, so a client streaming content as requests could never say
"that was the last one". What changed is what ties the two exchanges
together. `write_path` now carries a `write_id` the caller chooses, and
`write_bytes` quotes it back. A channel number was the wrong handle: it
belonged to whichever side minted it, so correlating by one meant one
side reading the other's numbering.

**A connector is not told the room's size.** Report 3 said a
connection's response "keeps its own count". It does not. A connector
learns the filetree and nothing else about who else is attached, which
leaves the runner as the only party that knows — and the runner is the
only one that authorized anybody. The count would have told a connector
about parties it never agreed to and cannot name.

## Nothing is acknowledged

The largest wire change since Report 3, and it removed frames rather
than adding them.

**The requestor chooses its own scope.** A scope used to be minted by
the server and handed back in an ack; now the client picks the number
and puts it in the request. Which took the ack's only job away — the
channel acks never had one, since a channel has always been numbered by
whoever opened it, so those two only ever said "received".

That is not worth a frame. A peer too busy to answer is a peer too busy
to acknowledge, so the signal is thinnest exactly when someone would
want it. A sender learns its request landed by being answered.

Seven types remain, and a number now means one thing in both
directions:

| type | client | server |
|------|--------|--------|
| 0 | auth | auth |
| 1 | request | — |
| 2 | — | response |
| 3 | — | response finish |
| 4 | channel request | channel request |
| 5 | channel response | channel response |
| 6 | channel response finish | channel response finish |

The blanks are what keep it that way. Only a client opens a scope and
only a server answers one, so `type` identifies a frame without
reference to who sent it.

**Every payload is bytes.** The frame layer no longer parses anything,
including auth, which it used to. What a scope request means is
`endpoints::ClientRequest`'s to say — the one place the ten tags meet,
since each is declared in its own module and nothing else can see them
all at once. Its decode is infallible: a payload nobody can name becomes
`Invalid` and is answered like any other request, because a server
holding bytes it cannot name has nowhere to complain but the connection.

## Auth is a mode and a string

Two modes. **Direct** is the two ends already knowing each other, and
the credential means whatever they agreed before either dialled.
**Brokered** is the mode that is not in this crate — `diverge-broker-sdk`
was bootstrapped for it, a second normative artifact for a third party
that answers who somebody is.

A string rather than bytes, because every credential this will carry
already is one. Bytes made a caller pick an encoding for something that
never needed one.

## One connection, either direction

`Connection` is incoming or outgoing — a socket this process's server
upgraded, or one it went out and made. That is a fact about TCP and says
nothing about which half of the protocol is spoken over it: a provider
usually waits to be dialled and sometimes dials a caller it cannot
otherwise reach.

It absorbs the difference between two libraries that agree on almost
nothing, and neither dials nor accepts. The features carry `axum`
without `http1` and `tokio-tungstenite` with `stream` alone — enough to
name both socket types and not to serve or connect with either. Where
the endpoint lives and what authenticates the upgrade stay outside.

## The caller half

A read loop and a writer, split because a `Sink` needs `&mut` and a read
loop never finishes. `Router` reads frames and forwards them whole,
header included, to whoever registered for them. `Handle` writes, mints
scope and channel numbers, and is cheap to clone.

Two queues run between them, both unbounded: registrations forward,
saying where frames should go **before** the request that causes them,
and closures back, saying a number is free again. A registration races
the answer it is for, which the router handles by draining whenever a
lookup misses.

A full queue **waits**. The alternative is dropping a frame, and a
stream has no way to say it lost one — a caller reassembling a layer
would get a shorter layer and no indication. A stall is visible and
recoverable; a hole is neither.

## The provider half is not a mirror

`Session` is a `Stream` of `ScopeHandle`s. It is not a router, and the
reason is the one asymmetry in the protocol: **a client mints scope
numbers and a server only ever learns them.** A client arranges where a
scope's frames will go before opening it, so something must hold those
arrangements — that is routing, and it is a job. Nothing on the server
can arrange a scope, so what would have been a router is a loop that
reads frames and hands out the ones that open something.

Channels go the other way. A channel this end opens is this end's to
number, and its answer needs somewhere to land before it arrives — so a
session routes for the half it opens and merely delivers for the half it
is told about. One queue carries the difference, and one queue rather
than two is load-bearing: on separate queues a registration can be
applied after the closure that should have covered it, and if the client
has meanwhile reused that scope number, it lands in the *new* scope's
map and shadows a live channel.

The write half belongs to the individual scope rather than the session,
which follows from the same asymmetry: a provider creates no scopes, so
there is nothing to share at that level.

One bug worth recording. Channel numbers were counted rather than
claimed, so a wrap could hand out a number still in use — and the
failure was silent and total, since the session's insert would overwrite
the live entry and route the old channel's answers to the new one. The
fix took the caller half's shape: a live set, and a number given back
only when the client **finishes** the channel. Not when the caller drops
it — that says a consumer walked away, which the client was never told,
so it may still be sending.

## Above the wire

`execute` performs an exchange rather than describing it, behind the
`client` feature, for the six scopes that are simple enough to be worth
it: an image check and all five volume operations. Everything else in
`endpoints` is still types only, for anyone who did not ask for the half
of the crate that can hold a socket.

Four of the six collapse into one call. `watch` does not — it sends a
snapshot and then changes for as long as the scope lives — so it hands
back a stream, and that stream is now generic. `ScopeResponseStream` and
`ChannelResponseStream` poll a receiver, strip both envelopes, and hand
the payload to one function the endpoint supplies. The only part of
reading a stream that ever differs is turning one payload into one item.

## Known gaps

- **Nothing has been executed.** No frame has round-tripped, now across
  a transport as well as ten scopes. This has been true in every report.
- **A provider cannot read what it was asked.** `ScopeHandle` can send
  every frame a provider has and expose none of what arrives: the
  opening request is held and unexposed, and channel requests land in a
  queue with no accessor.
- **Auth is discarded in both directions.** Both read loops drop the
  frame, and a provider on an outgoing connection cannot send the one it
  owes.
- **There is no cancel frame.** A client opens a scope and a server ends
  one; nothing lets a caller stop what it started. For a watch that is
  the *ordinary* exit, so a dropped stream leaves the provider sending
  and the scope number unreclaimed.
- **`connect`'s response documents its error as tag `2`; the constant
  says `1`.** A prose bug in the artifact that is supposed to be
  normative.
- **Variant order is wire-significant** in the postcard types.
- **`{ name, digest }` is defined twice** — in `images::check`'s request
  and in `Image`'s two pinned variants.
- **`shared::http::request::Request::body` is a `RawValue`**, so
  tunneled requests carry JSON bodies only.
- **`shared::http::response::Frame` owns a tag byte**, the only
  discrimination left in `shared`.
- **Deliberately unspecified:** what `bytes_used` measures, whether it
  can exceed `bytes`, what an `edit` below it does, and where `memory`
  is enforced given swap is unnamed.
- **Nothing delivers a connection's credentials.** A laboratory id comes
  from a run's own response and an authorization comes from nowhere at
  all — both reach a prospective connector out of band.
