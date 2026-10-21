# The order of work

The base is on main: the wire, the two handles, the exchange shapes
one answer, stream, stream with cancel and pending, the filetree and
listing folds, and the rules in `CLAUDE.md`. What remains is the
daemon's vocabulary and its one hundred and ten endpoints, plus the
two exchange shapes the base does not have, upload and the MCP
connect scope. This is the order they are built in. Each step is one
plan session, planned and landed before the next begins; a step
names what it depends on, and nothing is started out of order.

Every step is done the same way. The types are written against
`diverge-sdk-rs`'s source, module for module. Each request and
response schema is checked against what the CLI prints for it —
`diverge run <family> <request> request-schema` and
`response-schema`, which are the SDK's own JSON Schemas — and each
endpoint is run live against the local daemon as root, its answer
parsed by its schema. `typecheck` and `build` are clean, the live
run is recorded in the commit, and the step is committed by
pathspec and pushed.

## 1. The vocabulary

What every family's endpoints are made of, none of it an endpoint.
From `diverge-sdk-rs/src`: `daemon/creator` (who did a thing: a
client, an agent, a tool), `daemon/reference` (how a record is
named: an account by name or identity, an agent by template and
index, a tool, a volume by provider and name), `daemon/key` (agent,
tool, origin, owner — the keys the listing fold uses), `daemon/grant`
(every family's actions and the grants over them) with
`shared/permission`, `daemon/patch` (the JSON Patch operations every
edit carries), `shared/relay`, `daemon/download` (the chunk: a
`u32` big-endian length, the path as JSON, the body raw), and the
container-facing shapes under `shared/containers` that records
embed: dependency templates, the info document, the MCP shapes.

Depends on: the base. Done when every type an endpoint of steps 2
through 7 names exists, so that no later step writes a shared type
beside an endpoint.

**Done 2026-10-20.** Everything above, plus what the catalogue
showed the grants reach by: the twelve list filters, the provider
identity, the create request's pieces, the two template types, and
the provider's volume mode. Ninety-eight examples accepted and
rejected alike by the zod schemas and by the CLI's JSON Schemas.
Not vocabulary, left to their steps: the provider status and the
provider types it embeds (step 3), MCP and the attestation (step 6).

## 2. Accounts and roles

The two simplest families, fourteen endpoints, and the endpoint
pattern established once: `client/request.ts` with its `TAG` and
`encode`, `server/response.ts` with `decode` and, on a list, the
`Keyed` key and `change()`, `client/execute.ts` over the shape.
Create, get, list, delete, edit, tag, untag for each. Edit is the
first patch; list is the first listing folded live.

Depends on: step 1. Done when both families run live, including a
list kept open across a create and a delete and folded correctly.

**Done 2026-10-20.** Fourteen endpoints in the pattern CLAUDE.md
now states. Seventy-five examples accepted and rejected alike by
the schemas and by the CLI's JSON Schemas. Live as root: a roles
list held open saw the role added, changed as accounts took it, and
removed; an account created with a credential dialled in with the
key it was answered; edit, tag, untag, filtered lists, `exists`,
`in_use`, `not_found` all as the Rust states. One daemon bug found
and fixed on the way: a held role's delete answered `error` rather
than `in_use`, since PostgreSQL reports a RESTRICT refusal as
`23001`, which sqlx does not count as a foreign key violation.

## 3. The record families without containers

Providers outgoing, incoming and daemons, with daemons' accounts;
vault; postgres; events; and volumes' records — create, get, list,
delete, edit, tag, untag, stat. Forty-four endpoints, every one a
shape the base already has. Events is the one list without
`removed`; postgres the one with no filter; vault's insert and
delete name slots.

Depends on: step 2, whose pattern this repeats. Done when every list
of the seven families folds live and every unary answers.

**Done 2026-10-20.** Forty-one endpoints (the count above was
wrong by three), the provider-protocol `info` and `capacity`
answers the provider records embed, and the generator lifted into
a module every later family drives. One hundred and nine examples
agree between the schemas and the CLI's. Live, daemon and provider
together, with a `provider.yaml` written for the run (one fixed
credential, one volume store): an outgoing provider added and seen
connected on a list held open, with its version and capacity; a
volume created, listed, walked, tagged and deleted; an incoming
credential's key minted and rotated; a daemon record with an
account; the vault's slots appended, replaced, deleted and refused
as `last`; the database mode and the two empty lists. The one
answer not as asked: a volume's resize is the provider's honest
error on this host, `resize2fs` not being installed inside the
podman machine; a gap of the machine image, not of the SDK.

## 4. Download, upload and the filetree

The two shapes the base lacks, with volumes as their first user.
Download is a stream whose item is a chunk or an answer; upload is
a scope on which the DAEMON opens one channel per file and this side
answers each with content in pieces of at most `CHUNK_SIZE`, then
finishes it — `daemon/client/upload.ts`, a content source the caller
supplies. Transfer rides the same request shapes. The filetree is a
stream of postcard frames, decoded by the codec the base has and
folded by `Root`. Volumes download, upload, transfer and filetree:
four endpoints, three new shapes.

Depends on: step 3 (volumes' records). Done when a volume is
uploaded from the SDK, its tree watched, and downloaded back
byte-identical.

**Done 2026-10-20.** The content piece and the provider's write
content answer at their Rust paths; `daemon/client/upload.ts`, the
shape where this side answers the channels the daemon opens, each
file's content from a `Source` the caller supplies; the four
endpoints, the download's `Item` carrying the chunk's bytes beside
the JSON-form `Frame`, the filetree's tree decoded from postcard.
Fifty examples agree between the schemas and the CLI's, the CLI's
eight file leaves having been mended so that their schema
subcommands answer without `--path`. Live, daemon and provider
together, thirty-two checks: a file uploaded from a `Uint8Array`
and a directory from an async iterable whose one piece crossed
`CHUNK_SIZE`; the tree seen whole and at a subtree, a file's path
refused as `not_directory`; the directory and the file downloaded
back byte-identical, the large file in more than one chunk, the
paths in bytewise order; a directory and a file transferred into a
second volume and read back identical; a source that refused, and
one that threw partway, each the daemon's honest error and nothing
landed. Noted for the daemon, not the SDK: a transfer within one
volume is answered `held` (the read holds what the write asks
for), and for a few milliseconds after an errored upload the volume
is still reported held.

## 5. Agents and their templates

Nineteen endpoints. Templates: create, get, list, delete, tag,
untag. Agents: create, get, list, delete, edit, tag, untag, logs (a
stream with a count), message (the one pending answer), download,
upload, transfer, filetree. The template record embeds the
container shapes of step 1. This is the first family whose live run
needs a provider and an image; the run is made against the local
provider with the python image.

Depends on: step 4 (download, upload, filetree). Done when an agent
is created from a template, messaged, its logs streamed and its
filetree watched, from the SDK.

**Done 2026-10-20, without the live run.** Nineteen endpoints; the
attestation vocabulary in `shared.mcp`; the fourteen chunk kinds of
an agent's run at the provider's path, which the logs items are
made of; the MCP shapes themselves from `@modelcontextprotocol/core`
2.3.1, checked member for member against rmcp 3.1.2 before it was
taken as a dependency. Two hundred examples agree between the
schemas and the CLI's, the two that schemars cannot state — a
template's flat `type` — checked by zod alone. By the user's
ruling nothing was dialled: an agent created, messaged, its log
streamed and its tree watched from this SDK remains unproven, and
the first family to need an image and a running provider is still
waiting for that run.

## 6. Standalone tools, their templates, and connect

Twenty-three endpoints, and the last shape: `standalone_tools/connect`
is a long-lived scope on which THIS side opens channels, one per MCP
exchange — list tools, list resources, call a tool, read a resource,
notifications — and a disconnect; `daemon/client/connect.ts`. Also
register, attach and detach of agent instances and templates,
download, upload, transfer, filetree, and the templates' six.

Depends on: step 5. Done when a tool is created, connected, its
tools listed and one called, from the SDK.

**Done 2026-10-20, without the live run.** Twenty-three endpoints
and the last shape: the five MCP exchanges as `shared.mcp` frames
over MCP's own schemas, the server's error carried whole, the
provider protocol's channel readers mirrored at their path, and
`connect` — a scope this side opens channels on — handed back as an
`ExecuteHandle` whose methods are the exchanges. Two hundred and
thirty-four examples agree between the schemas and the CLI's, the
MCP frames by zod alone since the CLI prints no schema for a
channel's frames. By the standing ruling nothing was dialled: a
tool created, connected, listed and called from this SDK remains
unproven. Found on the way: the CLI's `connect call-tool` and
`connect read-resource` leaves cannot print their schema
subcommands, a required option in the way as the file leaves' was
before step 4 mended them.

## 7. Dependency tools

Nine endpoints: list, get, delete, tag, untag, filetree, download,
upload, transfer. Their records are keyed by template id and owner.
Nothing new in shape; the family closes the table at one hundred
and ten.

Depends on: step 6. Done when every tag in the daemon's table has
its folder, which a count of `endpoints/**/client/request.ts`
confirms.

**Done 2026-10-20, without the live run.** Nine endpoints in the
shapes the earlier steps set, the `Listed` record with the agents
that depend on it and its run, keyed by the template's id and the
owner. The table is whole: a count of `endpoints/**/execute.ts`
answers one hundred and ten, the connect's `execute/execute.ts`
counted once. Eighty-five examples agree between the schemas and
the CLI's. By the standing ruling nothing was dialled.

## 8. The proxy, proven, and the first publish

The proxy handle has only been type-checked: no Rust implements its
four commands yet. This step is the viewer's — a `diverge-viewer`
plan that holds a `diverge_sdk::wire::client::Handle` and implements
`wire_request`, `wire_channel_request`, `wire_channel_response` and
`wire_channel_response_finish` as thin calls on it — and the SDK's
side of it is running every shape of steps 2 through 7 through the
proxy handle unchanged, which is what the one-interface design
promises. Then the first publish: the `diverge` npm org made, the
README read once more as a stranger would, a release dry run.

Depends on: step 7, and the viewer's port to the current SDK, which
is its own branch. Done when `@diverge/sdk@2.3.0` is on npm and the
viewer lists accounts through it.

**Prepared 2026-10-20; not published, by ruling, and the viewer
untouched.** What was done: two stray files a generator had been
writing beside the tree since step 3 removed, with the cause; the
package shipping `src` beside `dist` so its maps resolve, and a
`bugs` address; the README read once more as a stranger would — an
install line that says why zod is named, a first exchange and an
upload shown end to end, `provider` and `shared.mcp` among what is
here, and a status that says what has and has not run live — its
snippets compiled against the built package; and the release
workflow's `@diverge/sdk` job gated on its own dispatch input,
`publish-diverge-sdk`, off by default, so a release run publishes
nothing here until asked. The pack dry run: 3,886 files, 4.9 MB
unpacked, 568 kB packed, nothing written and nothing sent. Left
open, as this step's other half: the proxy proven through the
viewer's Rust side, and the publish — one `pnpm publish --access
public` from `diverge-sdk-js`, or the workflow input, when the user
says so. A LICENSE file is the one thing a first publish might
still want; the sibling package ships none and the manifest names
MIT.
