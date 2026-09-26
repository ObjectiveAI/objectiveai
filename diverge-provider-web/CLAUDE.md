# Writing the Diverge Provider Protocol specification

This directory is the specification of the Diverge Provider Protocol,
published at `https://provider.diverge.network`. The specification is
the protocol's normative definition. `diverge-provider-sdk` implements
it, and each revision of the specification is the version of that
crate. This file governs how the specification is written. It binds
every page under `src/content/spec/`, and it binds me whenever I write
one.

## The register

The `spec-prose` skill (`skills/spec-prose/SKILL.md`, in this directory) is the
register every page is written in, and it is loaded before any page
is written or revised: complete sentences in the present indicative,
sequence stated by verbs of order and never by "then", every
quantifier exact, every value literal, no fragment, no contraction, no
commentary. What follows here is the document's shape; the skill is
its sentence.

Two conventions bind every page. A sentence that states what the
server or the proxy does is a requirement on the provider; a sentence
that describes what the client does states the input the server
serves and binds no client — the specification imposes no requirement
on a client, ever. A descriptive sentence that the page keeps — a
consequence, an example, an orientation — is set in a paragraph or
clause that begins with **Remark.** and is thereby marked as imposing
no requirement. `CONTRACT.md` at the root of this directory is the
conformance agreement that incorporates the specification; a
requirement the contract restates is stated on the page in the same
terms.

## The document

The specification is a scientific article. It is written the way a
statute or a standards document is written: every sentence states a
fact of the protocol, and every fact stated is a requirement. It has
no author's voice, no narrative, and no audience other than an
implementer who must be able to build a conforming party from the
text alone.

- **Only requirements.** A page contains the legal requirements of
  its layer and nothing else. It does not explain why a requirement
  exists, what it replaced, how the crate implements it, or what a
  reasonable implementation might do. A clause of reason is permitted
  only when it is itself a fact of the protocol on which the
  requirement's reading depends, as "there is no length field, because
  WebSocket already delimits messages." Motivation, history, design
  alternatives, and commentary on the crate are excluded without
  exception.
- **Declarative present indicative.** Requirements are stated as
  facts: "A frame never spans messages." "Only a client sends type
  `1`." The keywords of RFC 2119 are not used, because every sentence
  of the specification is already a requirement and marking some
  would unmark the rest. A permission is stated with "may". The
  protocol has no recommendations, so nothing is "should".
- **Precision over economy of words, economy of words over
  everything else.** A requirement is stated once, in the layer that
  owns it, in the fewest words that leave one reading. Nothing is
  restated in a higher layer; the higher layer links to it. Nothing
  is paraphrased in a summary; a summary is the requirement in
  miniature.
- **Every number is exact.** Byte widths, byte order, tag values,
  counts, and limits are stated as numbers, with their units and
  their endianness. "Nine bytes, fixed." "`u32`, big-endian." A value
  that is not prescribed is said to be not prescribed; it is never
  said to be zero.
- **Every term is defined once and used as defined.** The roles are
  *client* and *server*, fixed for a connection's life and independent
  of which party dialled. The parties are *caller* and *provider*. At
  the WebSocket, frame, and authorization layers only the roles are
  named. From the endpoints layer up, a caller is a client and a
  provider is a server, and the party words are used. *Scope*,
  *channel*, *request*, *response*, *finish*, *requestor*,
  *responder*, *ask*, *answer*, *exchange*, *stream* carry the
  meanings the frames layer gives them and no other. "Run", "loop",
  "container", "agent", "tool", "connector", "runner" carry the
  meanings the endpoints layer gives them. A word is never used in a
  loose sense on one page and a strict sense on another.
- **No filler.** No introductions that announce what follows, no
  transitions, no restatements at the end of a page, no "note that",
  no "it is important to", no hedging. A page begins with its first
  requirement's context and ends with its last requirement or its
  links.
- **Nothing is invented.** A requirement is written from the crate at
  the specification's revision, from its type declarations, its
  constants, and its documentation, and from the user's rulings. A
  gap in the crate is not filled by conjecture; it is reported to the
  user. Where a page's subject is not yet defined, the page carries
  `draft: true` and the banner that the crate is normative for it,
  and its body is empty. Where the crate and the specification
  disagree, the disagreement is reported to the user before either is
  changed.

## The notation

Two notations are normative, and each is used for exactly one kind
of payload.

- **Binary layouts** are written as bracketed field lists in a
  `text` code block, the order of brackets the order of bytes:

  ```text
  [type: u8][scope: u32, big-endian][channel: u32, big-endian][payload …]
  ```

  A width is a Rust integer type; multi-byte integers state their
  byte order; a run that continues to the end of the payload is
  written with `…`; a run whose length precedes it names the prefix
  (`[id_len: u16 BE][id…]`). Every binary layout is the layout of one
  message, and a page states which message.
- **JSON payloads** are written as Rust type declarations with serde
  attributes, under the reading fixed in the "Notation" section of
  `/endpoints/`. That section is the only place the reading is
  stated; every page that uses the notation is governed by it and
  does not restate it. A declaration is transcribed from the crate —
  field names, attributes, and types verbatim — and is never
  paraphrased into prose or a table. Types of `rmcp::model` are
  incorporated by reference to the Model Context Protocol
  specification and linked to docs.rs at the exact `rmcp` version the
  crate builds against, which the build reads from the crate's
  `Cargo.toml` and refuses to drift from.
- **Crate files are included, never transcribed.** Where a page
  shows a file of the crate — a declaration, a frame, an error — it
  shows the whole file, read at build time by an empty fence whose
  meta names the file relative to the workspace root:

  ````text
  ```rust include=diverge-provider-sdk-rs/src/container_proxy_endpoints/client_request.rs
  ```
  ````

  The sentence before the fence names the path. The build fails on a
  path that does not exist. The included text is the crate's, doc
  comments included, and is not edited on the page; a requirement the
  file does not state is stated in prose beside it. The dev server
  does not watch the crate: a change to an included file shows after
  a restart or a build.
- **Tables** enumerate: tag values, type values, kinds, paths. A
  table never describes a payload's shape, and a table's cells hold
  values and links, not requirements. A requirement that a table
  would carry is a bullet beneath it.
- **Streams are stated as sequences.** A schema constrains one
  message; the ordering and cardinality of messages on a scope or a
  channel is stated in prose: how many frames, in what order, what
  ends the sequence, and what a finish with nothing before it means.
  Every channel page states its sequence.

## The layers

The specification is divided into layers, lowest first. Each layer
defines what the layer beneath it leaves opaque, and no more. A page
belongs to exactly one layer and states only that layer's
requirements; what it needs from a lower layer, it links.

### Layer 1 — WebSocket (`/websocket/`)

The transport. It states the WebSocket version, the handshake as RFC
6455 defines it and that this protocol adds nothing to it, that no
subprotocol is negotiated and no extension required, that TLS is
permitted and its use is outside the specification, and that all
protocol data is binary data frames, text and control frames carrying
no meaning beyond RFC 6455's. It names nothing above the message.

### Layer 2 — Frames (`/frames/`)

The header and the multiplexing. It states the nine-byte header, the
seven type values and the direction each is defined for, the roles,
that a frame's kind is its type and direction alone, that undefined
types are malformed and disregarded without ending the connection,
that unread header fields are ignored and never prescribed, that
nothing is acknowledged, that a stream ends at its finish frame and
nowhere else with no timeout at this layer, and that payloads are
opaque here. Its sections: the client's frames, the server's frames,
scopes (a request opens one; a client mints its number; a scope's
number may be reused only after its finish; one request per scope;
the server answers with responses and one finish), and channels
(either party opens one inside a scope; each party numbers its own,
and the numbering spaces are distinct; one request per channel; only
the responder finishes a channel; a channel number is reusable only
after its finish; a finish with nothing before it is the protocol's
statement that the exchange could not be served). Payload contents
are not described at this layer.

### Layer 3 — Authorization (`/authorization/`)

The handshake in front of everything. It states which party
authenticates (the party that dialled), that the credential is the
connection's first frame and nothing precedes it, the one mode
defined (unbrokered: an opaque credential the other party judges),
that an accepted credential is followed by the connection simply
working and a rejected one by the connection's close with no frame
answering it, that a credential after the handshake is a protocol
violation the receiving party answers by ending the connection, and
that the identity a server derives from a credential is opaque and
is the identity every scope on the connection is served under.

### Layer 4 — Container proxy (`/container-proxy/`)

The external interface of the proxy binary a provider places beside
every container's entrypoint: not a layer of the protocol but the
de-facto API of a program, defined at the revision. It declares the
connection to be no connection of the Protocol; incorporates Layer 1
and Layer 2 onto it in full, with the provider's server as the client
and the proxy as the server — the two words the proxy layers use for
the two parties — and excludes Layer 3, so no authorization frame is
sent; and states only what is its own: the listener and its port,
that the server accepts exactly one connection in its life and
refuses a second, the begin scope first and once, the mounts before
the rest, what the connection ending means, and that nothing times
out. It restates nothing Layer 1 or Layer 2 states. The scopes
themselves are Layer 5's. It states nothing internal: not the
loopback listeners, not the pgwire listener, not the forwarding to
the program's own server, not `PORT`, not what a mounted file looks
like from inside, not how a write lands.

### Layer 5 — Container proxy endpoints (`/container-proxy-endpoints/`)

What a scope on the proxy connection is for: the six endpoints —
the two begins, the mount, the tree, the read, the write — their
tag table, and each endpoint's section shaped as a Layer 6 endpoint's
is: the request, the response as a sequence, the channels the client
opens (`client/`, titled `Client Channels`) and the channels the
server opens (`server/`, titled `Server Channels`), each channel its
own section with a request page and a response page. The client is
the provider's server and the server is the proxy, throughout. Every type shown is
the crate's file under `container_proxy_endpoints`, included, beside
the shared frame it aliases. What an ask means is stated once, on the
Layer 6 channel that relays it; this layer states only how it is
carried, and the one thing the proxy adds to what it carries: the
container's image under `_meta` on every MCP exchange, in either
direction, and on every chunk of an agent's conversation.

### Layer 6 — Endpoints (`/endpoints/`)

What a scope is for. It states the tag byte, the tag table (sixteen
endpoints: the three container scopes, the eleven volume endpoints,
the image check, the version), that an unreadable request is answered
by a bare finish, that growth is new tag values, and the Notation.
Each endpoint has its own section with, in this order, the request
(the payload after the tag), the response (the frames of the scope's
main stream, as a sequence: what may come, in what order, what ends
it), the channels the client opens, and the channels the server opens.
A channel is its own section beneath the endpoint, shaped as the
endpoint is: an index page stating the channel — who opens it, what it
carries, what answers it, what ends it — a request page stating the
payload of the channel request, and a response page stating the
sequence of channel responses, what ends it, and what a finish with
nothing before it means. The channels a party opens are grouped under a `client/` or a
`server/` section titled `Client Channels` or `Server Channels`, each
with the tag table; `containers::tools::connect`'s `client/disconnect` section is the
convention. The container scopes further state what
their id is, what ends the scope, and how a connect scope relates to
the run scope it joins.

### Layer 7 — Shared vocabulary (`/shared/`)

Payload forms that more than one endpoint carries, defined once and
linked from every channel that carries them: the container request
and its mounts; the error value; the OCI manifest and blob answers;
the authorize question and its answer; the tools declaration and its
answer; write content; the Postgres
pair; commands; the vault's five operations
and its lock rule; the five MCP exchanges; the seven FUSE operations
with the file-and-directory rule; the filetree, read, and write
forms; the schema form; the agent's loop, enqueue, and dequeue
forms. Each
form's page states its layout or declaration and the rules that are
the form's own — the vault's lock semantics, FUSE's no-retry rule and
the empty path — and nothing about which endpoint carries it; the
endpoint pages say that.

### Layer 8 — Container contracts (`/containers/`)

What an image must provide beside the proxy: every container's HTTP
server on `PORT`, with `/register` — answered with the tools the
program depends on — and `/schema` and their bodies and statuses the
same on either kind; the agent container's three paths
of the loop beside them; the tool container's MCP server at `/mcp`
beside them; that registration is once and before any loop or
exchange; what a fate and an outcome are. These are requirements on image authors, stated as such, and
the one place the proxy's other side is described.

## The versions

The specification is versioned, and a version is a module:
`src/content/spec/<version>/` holds the whole of one revision — its
overview, its layers, its sections — and nothing outside a module
belongs to any revision. The site around the modules (the pages, the
components, the layout, `spec.ts`, `include.mjs`, the build checks)
enumerates them and knows nothing of their text. Every revision looks
the same; only its text differs.

- **The latest module is the crate's version.** Its name is the
  version in `diverge-provider-sdk-rs/Cargo.toml`, and the build fails
  when no module has that name. Only the latest module is written to;
  every other is frozen.
- **Only the latest module includes.** An `include=` fence reads the
  crate at HEAD, which is the latest revision's crate; an include in
  any other module fails the build. When a revision is superseded,
  its includes are replaced by the text they resolved to, so the
  module stands on its own.
- **Links are version-relative.** A page writes `/container-proxy-endpoints/fuse-mount/`, never
  `/2.3.0/container-proxy-endpoints/fuse-mount/`; the build prefixes the module's version on the
  page and in the twin. So a new revision begins as a copy of the
  latest module under the new version's name, edited from there.
- **The revision is hard-coded where it is a value.** The version
  endpoint's answer is written as the literal string in the module's
  own pages, and the build checks that the latest module states the
  crate's version.
- **The version is the first path part.** `/2.3.0/websocket/`. The
  root redirects to the latest revision; a path without a version is
  a 404. The revision menu at the head of every page links the same
  page in every revision that has it, and a revision's root where it
  does not.

## The page

Every page is `src/content/spec/<version>/<layer>/<section>.mdx` with
frontmatter `title`, `summary`, `order`, `draft`. `order` alone sets
sequence; filenames carry no prefix. The summary is one sentence, a
requirement in miniature, YAML single-quoted, with `’` in place of an
apostrophe. The body is Markdown with the two notations above and
links; it uses no component beyond what the layout supplies, so the
page's `.md` twin at the same URL reads as the page does. A page has
one `h1`, supplied by its title. Links are root-relative,
version-relative, and end in `/`. A page links its lower-layer
dependencies and its adjacent sections; it does not link forward to
pages that do not exist.

## The site

Astro with `@astrojs/react`; React is authoring only, rendered to
static HTML; no JavaScript is shipped, and `pnpm build` runs
`scripts/verify-static.mjs`, which fails the build on any `<script>`,
resolves every `/llms.txt` link against `dist/`, and checks each
page's anatomy. No component carries a `client:*` directive. The
latest revision is read from `../diverge-provider-sdk-rs/Cargo.toml` at
build through `process.cwd()`. Dependency versions are never hand-written;
`pnpm add` records them. The Astro dev server is a daemon that caches
routes; it is restarted after a route file is added. The layout's
`<style>` is `is:global`.

`Containerfile` is the site's container, built from the WORKSPACE
ROOT (`podman build -f diverge-provider-web/Containerfile .`) because
the build reads the crate: a `build` stage runs the same `pnpm build`
gate on node 22, and a `serve` stage is unprivileged nginx on port
8080 holding `dist/` under `nginx.conf` — Markdown twins served as
`text/markdown`, trailing slashes canonical, a Content-Security-Policy
that permits no script. Base images are pinned by digest with the date.
`cloudbuild.yaml` builds that Containerfile from the repository root,
pushes it to Artifact Registry under `objectiveai/diverge-provider-web`
tagged `latest` and by commit, and deploys the commit's tag to the
Cloud Run service `diverge-provider-web` in `us-central1` on port 8080
with no environment and no secret; the service takes no configuration.

## The process

The specification is written one section at a time, the section
named by the user, in the order the user gives. A section is written
from the crate at the current revision and from the user's rulings,
checked by `pnpm build`, and committed by pathspec with the standard
trailers before the next is begun. A section already written is not
touched while another is being written, except to add a link that
the new section makes valid. When the crate changes under a written
section, the section is revised to the crate and the revision is
reported.
