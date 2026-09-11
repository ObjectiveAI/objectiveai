# Provider Protocol — Status Report 2

Report 1 described six scopes, two container kinds, and a filesystem
endpoint. There are now **ten scopes, three container kinds, and no
filesystem endpoint** — it became `volumes`, and grew a lifecycle.

The relationship has not changed: this crate *is* the protocol, prose
states requirements, and where they disagree the crate is correct.
Everything below was read out of the source rather than carried over
from the last report.

## The wire is unchanged

Nine-byte header, `[type: u8][scope: u32 BE][channel: u32 BE]`, no
length prefix. Nine frame types, `0` through `8`, the same table Report
1 printed. Auth still has no reply. The frame layer still parses
nothing.

One doc correction worth recording, because the crate was wrong where
Report 1 was right. `frame/mod.rs` claimed a channel was "a request the
SERVER made, and only the server opens them — which is why the two ends
can never collide over one." Both sides open channels, that is what
type `5` being shared means, and the reason they cannot collide is
per-sender numbering. The file contradicted itself two sections later
and contradicted `ClientFrame::ChannelRequest`, which already carried
the real rule. It now says what Report 1 said.

## Ten scopes

| tag | request |
|-----|---------|
| `0` | `agentic_loop::run` |
| `1` | `images::check` |
| `2` | `volumes::list` |
| `3` | `volumes::watch` |
| `4` | `laboratories::run` |
| `5` | `laboratories::connect` |
| `6` | `mcp_plugin::run` |
| `7` | `volumes::create` |
| `8` | `volumes::delete` |
| `9` | `volumes::edit` |

This table now lives in one place, `endpoints/mod.rs`, and every
request's `TAG` doc points at it. Report 1 listed "tag values allocated
across modules that do not know about each other" as a known gap, and
it had already rotted: six files each enumerated the others by hand,
and by the time there were nine requests one of them still said "so a
sixth request has to look at all of them." Adding a tenth is one edit
now.

They are in allocation order rather than grouped — `volumes` holds `2`,
`3`, `7`, `8` and `9`. Nothing derives meaning from adjacency, so
regrouping would change every implementation to make a table look
tidier.

## Every endpoint's scopes are named

Report 1's `agentic_loop` had `client` and `server` directly beneath
it. It now has `run`, and so does `mcp_plugin`; `laboratories::create`
became `laboratories::run`. So the shape is uniform: an endpoint is one
or more **named scopes**, and each scope splits by who sends.

`run` rather than `create` because the scope *is* the container's life
— holding it open is what keeps the container, ending it stops one. The
rename also settled a paragraph that was already false: the request
frame claimed "creating is not starting; running it is a separate ask."
There is no separate ask in this protocol and never was.

## MCP plugins — the third container kind

The largest addition. An agent runs in an `agentic_loop`'s container,
works inside a `laboratory`, and **calls** a plugin.

Nothing is injected into it. A laboratory gets an MCP server put there
by the provider, which is why the provider picks its port and hands it
a working directory. A plugin image already serves MCP — its own server
is the entrypoint and stays PID 1 — so the provider adds nothing and
only has to learn where to dial.

That removes three of a laboratory's fields and adds three. There are
no `mounts`, because a plugin serves tools rather than works on a
filesystem. There is no `initial_cwd`, because the image's own
`WORKDIR` governs its entrypoint and there is no second process to
place. There is no `name`. Instead:

- **`port`** — what the plugin's server listens on inside the
  container. The caller states it because only the image's author knows
  it. A wrong value is undetectable at creation: the container comes up
  fine and nothing answers.
- **`arguments`** — an ordered map of JSON values. Not more
  `environment`, because the values are JSON rather than strings; a
  plugin taking a number or a nested object would otherwise have every
  caller inventing an encoding.
- **`identity`** — ten fields saying on whose behalf it runs, fixed for
  the container's life, which is why it is in the request rather than
  on each exchange.

The answer on channel `0` is a **unit frame** meaning the plugin is up.
No id, because a plugin is not a place — nothing joins it and nothing
sends it files, so the scope is the whole handle. No filetree either,
because nothing would read one.

Three channels run outward from the provider: `Oci` to fetch a
caller-served image, `Postgres`, and `Command`. Two run inward from the
caller: `Mcp` and `Stop`.

**`Command` is new.** A plugin has no CLI binary in its container and
no daemon it may dial, so a command it wants run has to be run by the
caller. The plugin asks, the provider relays, the caller executes, and
the answers come back one frame per item until the channel finishes.
Both halves are opaque bytes — but for a different reason than
Postgres. Postgres must not be parsed, because parsing means tracking a
wire protocol. A command need not be parsed, because the vocabulary
belongs to the CLI, which gains subcommands on its own schedule. Which
also means a provider cannot refuse a command it disapproves of: what a
plugin may ask for is settled between the plugin and the caller.

**Postgres moved here from the agentic loop.** A plugin is what needs a
database; an agent talks to its tools, and a tool is what keeps
something. So the tunnel ends where the tool runs. `agentic_loop`'s
server channel request is consequently down to one thing — a struct
carrying an MCP exchange, with the tag byte kept so a second thing
stays additive.

## Volumes

`filesystem` became `volumes`, and `Directory` became `Volume`. The
module never described a filesystem: it describes directories a
provider has **decided to offer**, each under a name it chose, which is
what a volume is. It is also what a laboratory `Mount` names, so what a
caller can watch and what it can mount are one list.

`Volume` lost its `path`. A caller has nothing to do with a host path —
everything goes through the name, and a field nothing consumes is one
that gets consumed anyway. It gained `bytes`, `bytes_used`, and
`created` (seconds since the epoch, as an integer, because this rides
postcard and a date as text would be a text format inside a binary
one).

And volumes now have a **lifecycle**: `create` names one and states a
size, `edit` changes that size, `delete` destroys it. All three answer
with a unit frame and finish.

Which makes volumes the only thing in this specification that
**persists**. Every other scope owns what it made — a laboratory dies
with its connection, a plugin with its scope — and a volume survives
the scope that created it, every connection the caller holds, and every
container that mounted it. Only `delete` ends one. That is the point:
mount one into a laboratory, stop the laboratory, and the work is still
there.

## What the three container kinds now share

**`Image`**, one enum replacing the old `image_type` plus
`image_reference` pair. `Client` and `Server` carry a repository name
and a manifest digest — the same pair `images::check` asks about, so a
check that came back available names an image a run can ask for.
`Registry` keeps an arbitrary reference, because there the caller chose
the source and a reference is how a source is named. It is also the
only variant that need not be pinned, which is the point of it.

**`disk`**, a byte ceiling beside `memory`, on laboratories, plugins,
and the Python agent. It covers the container's own filesystem — what
it adds over the image, which is read-only and uncounted. A
laboratory's does not govern its mounts: a mount is storage that
already had a size a volume stated.

Both live in `shared::container::request`, a new sibling to `read`,
`write_path`, `write_bytes` and `transfer` — the part of asking for a
container that does not vary by kind, as against the part about using
one you already have.

Nothing anywhere states anything about **swap**, deliberately.

## Laboratories

Largely as Report 1 described, with one terminology fix. The party that
authorizes a connection is the **runner**, not the "creator": this
protocol never mints, carries, or references a creator identity, so the
docs were naming a party a reader cannot get hold of. "Runner" names a
position — whoever holds the run scope — which the protocol can express.

The `Authorize` channel request is unchanged and still carries both an
attested IP and an asserted opaque authorization.

## Known gaps

- **Nothing has been executed.** Still true, and now across ten scopes
  rather than six. No frame has round-tripped.
- **Variant order is wire-significant** in the postcard types. New
  variants go on the end; nothing enforces it.
- **`{ name, digest }` is defined twice** — in `images::check`'s request
  and in `Image`'s two pinned variants. The correspondence between them
  is the argument for `Image`'s shape, and it is currently a convention
  rather than a type.
- **`shared::http::request::Request::body` is a `RawValue`**, so
  tunneled requests carry JSON bodies only.
- **Deliberately unspecified, and worth knowing about:** what
  `bytes_used` measures, whether it can exceed `bytes`, what an `edit`
  below `bytes_used` does, and where `memory` is enforced given swap is
  unnamed.
- **Nothing delivers a connection's credentials.** A laboratory id
  comes from a run's own response and an authorization comes from
  nowhere at all — both reach a prospective connector out of band, by
  means this specification does not describe.
