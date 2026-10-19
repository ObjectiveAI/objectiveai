# Report 3: gets, the reach of a daemon tool, listing a runner's containers, descriptions, transfer, deployers and routes

Covers the fifteen protocol commits after report 2 on
`p2p-provider-binary`, from `aaf76ee74` on 2026-10-02 to `c542e8309`
on 2026-10-03, in `diverge-sdk-rs/src/daemon`, `diverge-sdk-rs/src/provider`,
`diverge-sdk-rs/src/shared` and the spec site. The two image-prompt
commits before them belong to report 2. The daemon's tag table grew
from thirty requests to thirty-seven; the provider's from sixteen to
seventeen.

| commit | what |
|---|---|
| `aaf76ee74` | `get` for agents, tools and both templates; `count` is `index` everywhere |
| `5cd22d1d2` | every daemon tool is a `Reach`, always present, `disabled` by default |
| `49df9c455` | a tagging tool is `Held`, each side `Within` |
| `ff17f7928` | `Reach`, `Held` and `Within` are flat on the wire |
| `5b7e30b04`, `52a11d677` | provider `containers::list` at 3 and an `authorize_list` channel on both runs; the site renumbered |
| `53187993e`, `3f738a96f`, `21141c465` | the pair is `AuthorizeConnect` and `AuthorizeList`, symmetrically |
| `b69ba7196`, `d908dfab7` | the listing is `containers::tools::list_for`; the agents run scope has no authorizations |
| `91289d214` | a resource has a required description |
| `f9aeecd15` | a template may carry a description, inside the hash |
| `54aadcf18` | the `transfer` tool replaces `resources_upload` |
| `c542e8309` | deployer agents, and tool routes at 25–27 |

## 1. Gets, and `index` (`aaf76ee74`)

- Four one-request endpoints, each right after its family's create:
  `agents::get` 1 and `tools::get` 16 take a reference by name or by
  template and index; `agents::templates::get` 10 and
  `tools::templates::get` 26 take an id. The answer is `Found` 0
  carrying the thing exactly as its list reports it, `NotFound` 1,
  `Error` 2.
- The instance ordinal is `index`, not `count`, on the `Agent` and
  `Tool` list items, on both creator structs, and on the reference
  variant, now `TemplateIndex { template, index }`. The list and logs
  requests keep `count`, which was always the cap.
- Every later tag shifted; the table reached thirty-four.

## 2. The reach of a daemon tool (`5cd22d1d2`, `49df9c455`, `ff17f7928`)

- `Reach<T>`: `"disabled"`, `"any"`, or what it names, flat on the
  wire — a filter, a pair of filters, a list of ids. `Disabled` is
  the default and what a member left off decodes as. Every member of
  `DaemonTools` is always present, so a patch replaces a member and
  never adds or removes one.
- `Switch`: `"disabled"` or `"any"`, for the tools with nothing to
  narrow.
- `Held<T>`: `"disabled"`, or `T` flat, for the eight tagging tools;
  no `any` at the root, because "any agent with any tag" is `T` with
  both sides open. Each side is a `Within<T>`: `"any"`, or what it
  names — a filter for which things, an array for which tags.
- The three are hand-written `Serialize` and `Deserialize` over one
  private `Word` enum; `Held` refuses `"any"` and `Within` refuses
  `"disabled"`.
- The create's `daemon_tools` is a `DaemonTools`, never an `Option`,
  left off the wire when every member is disabled.

## 3. Listing a runner's containers (`5b7e30b04` through `d908dfab7`)

On the provider.

- `containers::tools::list_for` at tag 3, after `tools::connect`. The
  request is an identity, `{"kind":"unbrokered","identity":…}`, one
  variant with a tag for the brokered one to come. The server asks
  the runner of every TOOL container that identity runs, on its run
  scope, and sends each container, `{"id":…}` at byte 0, the moment
  its runner says yes, in no promised order; the finish follows once
  every runner has answered or gone. A listing is FOR somebody else;
  agent containers take no connector and are never listed.
- `AuthorizeList` is the ask, carrying the lister's address and the
  identity its connection was authorized under, both attested; the
  answer is the existing one byte. `AuthorizeConnect` is the renamed
  connector ask, with its asserted authorization. The structs, the
  channel variants and tags, the response modules, the answer
  functions and the two `ConnectionAuthorizer` methods are named as a
  pair.
- Both authorizations are the tools family's: `AuthorizeConnect` at 3
  and `AuthorizeList` at 4 on `tools::run`, twenty-eight channels;
  the agents run scope has neither, twenty-six channels with `tools`
  at 3 and `fuse_setattr` at 25. `Own` carries no authorization; the
  connect and the listing build the tools family's own ask.
  `Directory::running_under` enumerates one runner's tool containers,
  and nothing else enumerates the map.
- Volumes shifted to 4–14, `images::check` to 15, `version` to 16.
- The site: new `containers-tools-list-for` and the two
  `authorize-connect` and `authorize-list` channel pages under the
  tools run, every renumbered endpoint and channel page rewritten in
  place, both server tables regenerated, the order check at zero
  findings, the build at 534 pages.

## 4. Descriptions (`91289d214`, `f9aeecd15`)

- A resource has a required `description` on both upload variants and
  on its list item. It is outside the hash: the same bytes uploaded
  twice are one resource, and the latest upload's description is the
  one kept, so an upload answered `Exists` has still said what the
  resource is.
- A template may carry `description: Option<String>`, second member
  after `type`, inside the hash so it travels: how to make a container
  from it, in words — the volumes and FUSE mounts it needs, the
  daemon's tools and their reach, which agents it has to message and
  which have to reach it. Both families get it through the one shape,
  and the list and get items carry it inside `template`.

## 5. Transfer (`54aadcf18`)

`transfer: Reach<Vec<Edge>>` replaces `resources_upload`. An
`Edge { from: Source, to: Destination }` passes a transfer when both
ends match; a list of edges is the reach, and direction is the
permission. A `Source` is `own` with optional path prefixes, `tools`
narrowed by the tools list filter as a `Within` with optional path
prefixes, or `resources` as a `Within` of ids. A `Destination` is the
same `own` and `tools`, or a bare `resources`: a transfer there is an
upload, making a resource of what it carries with the description the
call gives, made BY the container. Per-tool paths are separate edges.

## 6. Deployers and routes (`c542e8309`)

- `deployer_agent: Option<reference::Agent>` on both creates after
  `daemon_tools`: the agent the daemon hands the container's declared
  tool dependencies to, each as its template and instructions, when
  no route answers them. The deployer makes the tool, attaches it,
  and may add a route. Absent, a dependency no route answers is unmet
  and the container's tools channel is answered with an error. Both
  list items report it as a `creator::Agent`.
- `tools::routes`: a `Path { agent, templates }` is one dependency
  position — the agent that began the chain by name, the template ids
  down the chain, the last the dependency's own. A path has at most
  one route, and only a tool made from the path's last template may
  be routed there.
  - `add` 25: `{ path, tool }` → `Added`, `NoTool`, `Mismatch`,
    `Exists`, `Error`.
  - `delete` 26: `{ path }` → `Deleted`, `NotFound`, `InUse` while an
    active container is served through it, `Error`.
  - `list` 27: a filter over the starting agent, the ending template,
    the target tool, creators and a created span, with `jq` and
    `count`; a stream of `Route { path, tool, created, creator }`.
- The `Tool` list item gains `routes`, the paths routed to it. Tool
  templates shifted to 28–33, resources to 34–36.
- `DaemonTools` gains `tools_routes_add`, reaching the tools a
  dependency may be routed to, and `tools_routes_delete` and
  `tools_routes_list`, by the routes list's filter. Thirty-two
  members.

## The daemon's tag table at `c542e8309`

| tags | requests |
|---|---|
| 0–8 | `agents::{create, get, delete, message, logs, list, edit, tag, untag}` |
| 9–14 | `agents::templates::{create, get, list, delete, tag, untag}` |
| 15–24 | `tools::{create, get, edit, connect, attach, detach, delete, list, tag, untag}` |
| 25–27 | `tools::routes::{add, delete, list}` |
| 28–33 | `tools::templates::{create, get, list, delete, tag, untag}` |
| 34–36 | `resources::{upload, list, delete}` |

## The provider's tag table at `c542e8309`

| tags | requests |
|---|---|
| 0–3 | `containers::agents::run`, `containers::tools::run`, `containers::tools::connect`, `containers::tools::list_for` |
| 4–14 | `volumes::{list, stat, read, write, filetree, serve, create_capacity, create, edit_capacity, edit, delete}` |
| 15, 16 | `images::check`, `version` |

## Open

- The provider's `connect` has no daemon tool, by ruling.
- What each daemon tool says to its caller, and how the daemon knows
  which container is calling, is not yet stated.
- Editing an agent or a tool as a JSON Patch over an editable
  document, with the fixed set of daemon tools and the editable
  reach, was discussed and not built.

## Verification across the fifteen

Every commit passed `cargo check -p diverge-sdk` and
`cargo doc --no-deps -p diverge-sdk` with zero warnings, and the
provider commits `cargo check -p diverge-provider`, plus a
throwaway round-trip example asserting the tag values and the JSON
shapes, deleted after. The site commits passed the order check and
`pnpm build`. Nothing was run live. The remote branch was rewritten
once during this span, with the file
`diverge-agentic-loop-cc/reports/tools.md` purged from its history;
the work was rebased onto the rewrite rather than force-pushed, and
the old history is kept locally at `backup-before-rebase`.
