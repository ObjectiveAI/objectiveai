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

## 3. The record families without containers

Providers outgoing, incoming and daemons, with daemons' accounts;
vault; postgres; events; and volumes' records — create, get, list,
delete, edit, tag, untag, stat. Forty-four endpoints, every one a
shape the base already has. Events is the one list without
`removed`; postgres the one with no filter; vault's insert and
delete name slots.

Depends on: step 2, whose pattern this repeats. Done when every list
of the seven families folds live and every unary answers.

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

## 6. Standalone tools, their templates, and connect

Twenty-three endpoints, and the last shape: `standalone_tools/connect`
is a long-lived scope on which THIS side opens channels, one per MCP
exchange — list tools, list resources, call a tool, read a resource,
notifications — and a disconnect; `daemon/client/connect.ts`. Also
register, attach and detach of agent instances and templates,
download, upload, transfer, filetree, and the templates' six.

Depends on: step 5. Done when a tool is created, connected, its
tools listed and one called, from the SDK.

## 7. Dependency tools

Nine endpoints: list, get, delete, tag, untag, filetree, download,
upload, transfer. Their records are keyed by template id and owner.
Nothing new in shape; the family closes the table at one hundred
and ten.

Depends on: step 6. Done when every tag in the daemon's table has
its folder, which a count of `endpoints/**/client/request.ts`
confirms.

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
