# Provider Protocol — Status Report 8

Report 7 ended with one word remaining, and the word was
**authorization**. It landed — both halves, four commits after the
report that named it. Then something larger happened than any single
feature: the specification moved out of the crate. The protocol now
has a normative document that is not Rust, the crate became that
document's first implementation, and the next work is the first
containers — which is to say, the first code that will ever sit on the
far side of this protocol's wire.

## Authorization, both halves

The wire had `Auth` designed since before the handlers existed — frame
type 0, the connection's first frame, sent by whichever side dialled,
never answered — and nothing sent or read it. Now both halves do both.

**The server half is the door.** `UnbrokeredAuthorizer` is the fifth
thing a provider supplies: every other trait is asked things on behalf
of a caller, and this one decides who the caller IS. `Ok` is the
`client_identity` everything downstream has taken on faith since the
handlers arrived — this is where it finally comes from — and `Err` is
a refusal that goes nowhere on the wire, surfacing only in what
`handle` returns, because an unauthenticated peer earns no bytes.

`Authorization` replaced `handle`'s `client_identity` parameter and
encodes what that parameter hid: where an identity comes from depends
on which side dialled. `Incoming` carries the authorizer and demands
the `Auth` frame first, before any dispatch; `Outgoing` carries an
owned credential to present and the identity beside it, because the
provider chose whom to dial and that choice is the identification. An
enum, so a brokered mode is a new variant rather than a signature
break — the credential envelope reserves mode byte `1` for it, with
mode `0` the unbrokered UTF-8 credential in use today.

**The client half is the mirror, with the dial inverted and no
identity anywhere.** A caller judges or presents a credential but
derives nothing from it: its `UnbrokeredAuthorizer` answers `Ok(())`
where the server's answers with who the peer is, and its `Outgoing`
carries only the credential. The structure differs where the
architecture forces it — the server's handshake lives inside `handle`
because that function owns the whole connection; the client has no
such function, so `client::authorize` is a standalone step on the
UNSPLIT `Connection`, run before `Router` and `Handle` are built from
it. In both directions an `Auth` frame after the first is a protocol
violation, and `Router::run` now returns `Result<(), InvalidAuthorize>`
to say so.

Report 7's one structural gap — "auth is discarded in both
directions" — is closed.

## The vocabulary settled

Three renames of record, breaking on the wire and taken deliberately
at 2.3.0:

- The agent kind `claude_agent_sdk` is **`claude_code`**, and
  `codex_sdk` is **`codex`** — module names, variant names, and the
  `upstream` discriminant strings alike.
- OpenRouter's `synthetic_reasoning` parameter is gone.

`version.sh` now versions the website alongside the four SDKs, and
everything sits at **2.3.0**.

## The specification is a website now

Report 7 called this crate "the specification those two are checked
against." That sentence is retired. The protocol's normative
specification is **diverge-provider-web**, and it will be served at:

**https://protocol.diverge.network**

The protocol is the protocol; the Rust happened to come first. This
crate is now the specification's first implementation, checked against
it and not the reverse.

The site is React authored, Astro rendered, and ships **zero
JavaScript** — enforced by a post-build verifier that fails the build
on any `<script>`, alongside checks for canonicals, exactly one `<h1>`
per page, and a Markdown twin at every page's own path. It is built to
be read by machines as much as people: `llms.txt`, a sitemap, and
`.md` twins throughout. The revision on every page is the crate
version, read from this repository's `Cargo.toml` at build time.

What is written, as of 2.3.0, in 39 pages:

- **Overview**, **WebSocket** (the substrate declaration), **Frames**
  (client, server, scopes, channels — with the ignored-fields
  doctrine: unread header fields are IGNORED, no value prescribed),
  and **Authorization** (the first-frame law and the unbrokered
  credential).
- **Endpoints** — the tag table for all eleven, and tag `0`,
  `agentic_loop::run`, complete: Request with the four agent kinds as
  sections (OpenRouter, ClaudeCode, Codex, Python), Container,
  Response with all ten chunk declarations, and Channels with the five
  MCP exchanges, each with its own request and response sections.
- Payload shapes are stated as **Rust declarations under a normative
  notation** — the notation defines the JSON, binds no implementation
  language, and reads serde attributes as law. MCP types are
  incorporated by reference to the MCP specification, and every
  referenced `rmcp::model` type links to its definition, pinned to the
  rmcp version this crate builds against; the site's verifier reads
  that version out of this repository's `Cargo.toml` and fails the
  build if any link drifts.

Ten endpoints remain, to be written in tag order.

## Next: the first containers

The **Container** section of `agentic_loop::run` now specifies what a
server deploys: one image per agent kind, the memory and disk
ceilings, and three in-container ports — `8080` the loop (the
caller's request in over HTTP, the chunks back as an SSE stream),
`8081` MCP (the container listens; the server connects and answers
the agent's tool calls by relaying them as channel exchanges), and
`8082` Postgres (one TCP connection per database session, pgwire
relayed verbatim, routed to the caller).

The immediate work is to build the first two of those images: the
**OpenRouter** and **ClaudeCode** agentic_loop containers. They are
the point of the section's current shape — building them is what
refines the container specification from stated to proven, and the
Postgres contract in particular is written to be corrected: the
`8082` behavior is specified ahead of any implementation, and the
containers are how we will learn what routing a live pool of pgwire
sessions actually requires. The image references and resource numbers
the SDK carries as constants get settled by the same work.

## Standing gaps

- **The hard-coded numbers** — image references and the memory and
  disk ceilings — are now stated in the specification as they are
  encoded here, and both get revised together when the containers
  exist. The two agentic_loop ports are no longer a gap: the
  specification fixed them, with a third beside them.
- **No channel-level cancel**, **`write_id` uniqueness unpoliced**,
  **variant order wire-significant in the postcard types**,
  **`{ name, digest }` defined twice** — unchanged from Report 7.
- **Nothing has been executed** — still true, and the containers are
  the beginning of its end: they are the first artifacts that will
  face this crate from the other side of a socket.
