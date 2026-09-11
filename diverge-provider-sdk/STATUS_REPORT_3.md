# Provider Protocol — Status Report 3

Two things in this one. A correction, because Report 2 described a
protocol whose file operations it never introduced; and failure, which
is what most of the work since has been about.

The crate is also now `diverge-provider-sdk`, ahead of the product
rename. Nothing outside it moved.

## The correction

Report 2 referred to `read`, `write_path`, `write_bytes` and
`transfer` as though a reader already knew them — "a new sibling to
`read`, `write_path`, `write_bytes` and `transfer`" — and listed a
gap in a channel response it had never described. Report 1 was right
not to mention them: they did not exist. They landed **between** the
two reports, which made them Report 2's to introduce, and it skipped
them.

Three smaller things fell in the same window and the same gap: `Stop`
and `Disconnect`, the `name` a laboratory carries, and the connector's
address on an `Authorize`. All are described below. Report 2 is left as
it stands, the same way Report 1 was — a numbered report is a snapshot,
including of what its author missed.

## Reaching into a container

Both laboratory scopes let a caller act on the container's filesystem,
and the four exchanges are defined once in
`shared::container` because a run and a connection do the same things
once attached. What differs is only how they got in.

**`read`** names a file by path components and streams it back. One
file, never a directory — not a limitation so much as an admission,
because nothing walking a filesystem from userspace can snapshot one.
`tar` is the obvious counter-example and is not one: it makes the same
syscalls, holds no privilege, and prints `file changed as we read it`
precisely because it detects what it cannot prevent.

There is **no length anywhere** — not in a head, not in the request,
not implied. A file being written to changes size in both directions
after a sender has looked at it, so a length stated up front is a
promise about a number that has already moved. That commitment is what
makes `tar` corrupt a whole archive when one entry shifts.

**A write is two exchanges**, and that is the part worth explaining.
`write_path` names the destination and carries no content;
`write_bytes` is a channel the **provider** opens to ask for it, and
the content arrives as responses on that.

The inversion is forced. There is no request-finish frame in this
protocol — only a responder ends a channel — so a client streaming
content as repeated requests would have no way to say "that was the
last one" and would have to invent an end marker in the payload. One
round trip buys a channel that does exactly one thing.

**`transfer`** moves a file between two containers on one provider
without either direction crossing the wire. Its source is the container
the scope is attached to, so it names a path, a destination id and a
destination path — never two locations, because one of them is already
known. On a filesystem with reflinks it is extent sharing and no bytes
move at all. It is unavailable between providers, and nothing here
pretends otherwise: two providers have no filesystem in common.

Piping a read into a write is the expected use and needs no buffering.
A read body is one frame; it goes out as one write-content frame, never
copied, only re-tagged.

**`Stop` and `Disconnect`** end a scope deliberately. Both are
unanswered — what comes back is the scope's own finish — and both exist
because a provider cannot tell a deliberate exit from a network that
stopped answering, and would otherwise have to wait to find out.
`Stop` ends the laboratory and every connection to it; `Disconnect`
ends only the leaving connector's own scope.

## Errors

The single largest change since Report 2, and the shape is one type:

```rust
pub struct Error(pub serde_json::Value);
```

A JSON value and nothing else. A typed enum would have to name what a
container runtime, a kernel, a filesystem, a registry and an upstream
model can each go wrong with, and would be wrong the first time any of
them added one. What a caller does about most of them is the same
anyway.

It lives in `shared::error` because a failure meaning different things
in different modules is one every consumer learns twice. **Where it
appears does not**: whether an exchange can fail is endpoint logic, so
the enum that adds the variant lives in the endpoint, wrapping the
shared success type. Eighteen frames carry one: all ten scope
responses, and the eight file-operation channel responses — `read`,
`write_path`, `write_bytes` and `transfer`, in each laboratory scope.

Two constraints came with it. `serde_json::Value` deserializes through
`deserialize_any`, which a format with no self-description cannot
answer, so an error decodes from JSON and never from postcard. Where a
success variant is postcard — a volume listing, a filetree frame — the
tag chooses the format one variant at a time, and four responses
needed a separate encode-error type because postcard's failure and
`serde_json`'s are different types.

**The other five kinds of channel deliberately have none.** `mcp` and
`oci` are HTTP, which already has failure semantics; a second one would
be two error channels over a protocol with one. `command` carries the
CLI's own vocabulary, which has an `Error` inside the bytes this
protocol does not read. `postgres` is a byte pipe whose only two
signals are bytes and EOF — pgwire reports its own errors as ordinary
messages, and the far end is a driver that understands nothing else.
And an `authorize` answer cannot fail: a runner with nothing to say
denies.

## A stream ends at its finish frame

Stated once, in `frame`, because it holds everywhere. A quiet channel
is a channel still running, however long it has been quiet. Nothing
times one out and a reader waits.

Which retired an idea that had crept into the write docs — that a
client stopping mid-stream was a way to cancel. It is not. A sender
that stops without finishing has not cancelled anything; it has left a
stream open, and a sender with something to say says it in a frame.

`read` lost its `Corrupted` variant to the same tidying. A provider can
still detect a file written underneath a reader and still cannot
prevent it; what it no longer has is anywhere to report it. `transfer`
has always been in that position, and now they read the same way.

## Nicknames

A laboratory's run response no longer reports a connector **count**. It
reports a **departure**, carrying the nickname the runner gave that
connector when it authorized it.

A count needs no identity because it can be re-sent and read afresh. A
departure cannot: "one of them left" is not an answer to "which", and a
runner holding four authorized connectors could not tell. So the
authorize answer stopped being a bare bool and became a denial or an
authorization with an optional name, and the provider hands that name
back on the way out. It never reads it, and the connector is never told
it has one.

A connection's response keeps its own count. A connector counting the
room is a different question from a runner tracking who it let in.

## The loop's error became a notification

`ErrorChunk` is now `NotificationChunk` — anything the loop has to say
about itself rather than about what the agent produced. Its HTTP status
code is gone, replaced by `is_fatal`.

Fatal rather than merely wrong, because "wrong" is not a question a
caller can act on and "over" is. A provider that hits an error and
retries past it has not failed, and a caller told otherwise would
abandon a run that was still going.

## Known gaps

- **Nothing has been executed.** No frame has round-tripped, across
  ten scopes and now considerably more variants.
- **Variant order is wire-significant** in the postcard types.
- **`{ name, digest }` is defined twice** — in `images::check`'s
  request and in `Image`'s two pinned variants.
- **`shared::http::request::Request::body` is a `RawValue`**, so
  tunneled requests carry JSON bodies only.
- **`shared::http::response::Frame` owns a tag byte**, the only
  discrimination left in `shared`. Judged a special case rather than an
  oversight: head-versus-body is HTTP's own shape, identical in all six
  users, and moving it would mean six identical enums. If an `mcp` or
  `oci` channel ever gains an error, that judgement has to be revisited.
- **Deliberately unspecified:** what `bytes_used` measures, whether it
  can exceed `bytes`, what an `edit` below it does, and where `memory`
  is enforced given swap is unnamed.
- **Nothing delivers a connection's credentials.** A laboratory id
  comes from a run's own response and an authorization comes from
  nowhere at all — both reach a prospective connector out of band.
- **A nickname is not unique and nothing says it should be.** Two
  connectors may share one, and a departure then names both.
