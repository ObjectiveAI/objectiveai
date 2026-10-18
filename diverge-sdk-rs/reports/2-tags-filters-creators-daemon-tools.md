# Report 2: tags, narrowed lists, creators, references, and the daemon's own tools

Covers the fifteen commits after report 1 on `p2p-provider-binary`,
`01750870f` through `e379acebb`, all on 2026-10-01 and all in
`diverge-sdk-rs/src/daemon`. Every one is a wire change. The tag
table grew from twenty-two requests to thirty.

| commit | what |
|---|---|
| `01750870f` | tags as create-time members (reverted by the next) |
| `457d4abb8` | tags as endpoints: tag and untag for all four families; list items report them |
| `d50fc0d36` | the four lists narrowed: candidates, state, tags, a span, a jq program, a count; the response is a value |
| `cea78640c` | `tags` splits into `all_tags` and `any_tags` |
| `2a1a72826` | an agent's `count` among those made from its template; a first `creator` |
| `6f5e56617` | `daemon::creator`: every list item carries its creator chain; lists narrow by `creators` |
| `1c658d26f` | creator variants are structs of their own; a tool creator is named by template and count |
| `ba769cd53` | `daemon::reference`: every request on an agent or a tool names it by name or by template and count |
| `338f24a9b` | `daemon::builtin`: an agent template names the daemon's tools its agents hold |
| `dd4292e11` | `builtin` is `daemon_tools`, on the one template for both families |
| `c0ba30a17` | each list's filter is a `Filter` of its own; twenty daemon tools, each reaching what its filter passes |
| `19dd12cca` | a single-filter tool is that filter on `DaemonTools` |
| `3fcb01fb7` | the tagging tools carry which tags they may act with |
| `30cd31da1` | parity: template lists, template tagging, resources; twenty-nine tools |
| `e379acebb` | `daemon_tools` moves from the template to the two creates |

## 1. Tags (`457d4abb8`)

A tag is a string of the caller's choosing, compared and not read,
as a name is. What an agent, a tool, an agent template or a tool
template holds is a set of them.

- Eight endpoints, one request and one answer each: `agents::tag`
  6 and `untag` 7; `agents::templates::tag` 11 and `untag` 12;
  `tools::tag` 20 and `untag` 21; `tools::templates::tag` 25 and
  `untag` 26. The request is `{ name, tags }` for an agent or a
  tool and `{ id, tags }` for a template. The answer is `Tagged` or
  `Untagged` 0, `NotFound` 1, `Error` 2.
- Tagging a held tag, untagging an absent one, or an empty list,
  changes nothing and is not a failure.
- The four list items report `tags`, sorted bytewise, absent when
  empty. A template's tags are the caller's, beside the template in
  its list item and not in its hash.
- Later tags shift: agent templates to 8–10, tools to 13–19, tool
  templates to 22–24, resources to 27–29. Thirty requests.

The create-time version of the day before, `01750870f`, put `tags`
inside the template and on the creates and connect; it was reverted
in the same commit that added the endpoints.

## 2. Narrowed lists (`d50fc0d36`, `cea78640c`, `c0ba30a17`)

The four list requests, modelled on `agents::logs`, every member
optional and `{}` still the whole list.

- Agents: `names`, `templates`, `active`, tags, a `created` span,
  `jq`, `count`.
- Tools: `names`, `templates`, `kind` (`created` or `connected`, a
  new `Kind` enum), `active`, `agents` (attached to every one named),
  tags, the span, `jq`, `count`.
- Both template lists: `ids`, `in_use`, tags, the span, `jq`,
  `count`.
- Candidate lists match any one of; `all_tags` every one of;
  `any_tags` any one of; `agents` every one of. An empty list is
  absent and matches everything.
- The filter runs first, oldest created first; the program runs over
  each survivor and what it yields is sent; `count` caps what is
  sent. A program that will not compile or fails is the scope's
  error.
- The response is `Value` 0 and `Error` 1, the logs response's
  shape. The typed items `Agent`, `Tool` with `Origin`, and the two
  `Listed` stay public beside each frame as the reference for what a
  value is without a program.
- In `c0ba30a17` every member but `count` moved into a `Filter`
  struct beside each `Frame`, flattened into it, so the wire is
  unchanged and the daemon's tools can carry the same shape.

## 3. Counts and creators (`2a1a72826`, `6f5e56617`, `1c658d26f`)

- `Agent.count`: the agent's number among all of the caller's agents
  ever made from that template, deleted ones included, starting at 1
  and never repeated. `Tool.count` the same, per template for a
  created tool and per joined container for a connected one.
- `daemon::creator`: `Creator`, tagged `type`, with newtype variants
  `Client(Client { identity })`, `Agent(Agent { template, count,
  name })` and `Tool(Tool { template, count, name })`, each struct in
  its own file.
- Every list item carries `creator: Vec<Creator>`, required and
  never empty: the client first, each next made by the one before,
  the last made the item. A thing the client made has a chain of
  one. A template made again keeps its first maker's chain.
- A connected tool is somebody else's container and calls nothing of
  the client's, so it makes nothing and is never a creator; the
  earlier `ToolOrigin` was removed for that reason.
- Every list `Filter` gains `creators`: naming a creator matches
  everything made under it anywhere in its chain.

## 4. References (`ba769cd53`)

`daemon::reference::{Agent, Tool}`: an untagged enum of two objects,
`{"name":…}` or `{"template":…,"count":…}`, with members of both
refused at decode. By name reaches the thing now; by template and
count reaches it once and for all and finds nothing after deletion.
A connected tool is reached by name alone.

The twelve requests that act on an agent or a tool carry one in
place of the string: agents delete, edit, message, logs, tag and
untag under `agent`; tools edit, delete, tag and untag under `tool`;
attach and detach under both. Creates and connect still take a plain
`name`, since there the name is being given.

## 5. The daemon's own tools (`338f24a9b` through `e379acebb`)

`daemon::daemon_tools`: the verbs the daemon itself answers as MCP
tools, over the caller's agents, tools, templates and resources.
`DaemonTools` names which of them a container holds and how far
each reaches, one member per endpoint except `tools::connect`,
twenty-nine in all. A member absent withholds the tool.

- **Where it rides.** First on the agent template as `builtin`
  behind a flattened `AgentKind`; then renamed `daemon_tools` and
  put on the shared `Template` for both families; finally, in
  `e379acebb`, moved off the template onto `agents::create` and
  `tools::create`, before `name`. It is the container's for its
  life, and a template hashes without it, so one template makes
  agents of different reach. A connected tool holds none.
- **The filter as a permission.** A member's reach is the family's
  list `Filter`, read as a test: a thing is within reach when it
  passes every member given and, with a program, when the program run
  on it yields first a value that is neither `false` nor `null`. A
  program yielding nothing or failing passes nothing, and transforms
  nothing. `{}` reaches everything of the caller's. A list tool
  answers only what its filter passes, narrowed further by the
  request's own filter; a create reaches the templates it may make
  from.
- **Members.** Bare filters for `agents_list`, `agents_message`,
  `agents_logs`, `agents_create`, `agents_delete`, `agents_edit`,
  `agents_templates_list`, `agents_templates_delete`, `tools_list`,
  `tools_create`, `tools_edit`, `tools_delete`,
  `tools_templates_list`, `tools_templates_delete`. Booleans for
  `agents_templates_create`, `tools_templates_create`,
  `resources_upload`. `ToolsAttach` and `ToolsDetach` carry a filter
  for the tools and one for the agents. The eight tagging tools
  (`AgentsTag`, `AgentsUntag`, `AgentsTemplatesTag`,
  `AgentsTemplatesUntag`, `ToolsTag`, `ToolsUntag`,
  `ToolsTemplatesTag`, `ToolsTemplatesUntag`) carry a filter and
  `Tags`, `"any"` or `{"only":[…]}`. `resources_list` and
  `resources_delete` carry `Resources`, `"any"` or `{"only":[ids]}`.
- The earlier single-field wrapper structs and the one-variant
  `Tags { Free }` enum were removed on the way.

## The tag table at `e379acebb`

| tags | requests |
|---|---|
| 0–5 | `agents::{create, delete, message, logs, list, edit}` |
| 6, 7 | `agents::{tag, untag}` |
| 8–10 | `agents::templates::{create, list, delete}` |
| 11, 12 | `agents::templates::{tag, untag}` |
| 13–19 | `tools::{create, edit, connect, attach, detach, delete, list}` |
| 20, 21 | `tools::{tag, untag}` |
| 22–24 | `tools::templates::{create, list, delete}` |
| 25, 26 | `tools::templates::{tag, untag}` |
| 27–29 | `resources::{upload, list, delete}` |

## Open

- What each daemon tool says to its caller, and how the daemon knows
  which container is calling, is not yet stated.
- The agent and tool list items do not repeat `daemon_tools`, as
  they do not repeat the create's mounts.
- `tools::connect` has no daemon tool, by ruling.

## Verification across the fifteen

Every commit passed `cargo check -p diverge-sdk` and
`cargo doc --no-deps -p diverge-sdk` with zero warnings, plus a
throwaway round-trip example asserting the JSON shapes and the tag
values, deleted after. Nothing was run live.
