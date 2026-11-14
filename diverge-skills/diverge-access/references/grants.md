# Every grant kind, every action

A grant is `{"<kind>": <permission>}`. The permission's shapes:

- **Make**: `["<make action>", …]`
- **Over**: `{"actions":["<over action>", …], "within": "any" | <filter>}`
- **Tagging**: `{"actions":["tag"|"untag", …], "within": "any" | <filter>, "tags": "any" | ["<tag>", …]}`
- **The one thing** (`postgres`, `events`, `images`): `["<action>", …]`

An administrator (an identity under `admins:` in `daemon.yaml`) is allowed everything and needs no grant.
`diverge run info` needs no grant. Print the authoritative schema with
`diverge run roles create request-schema --jq '."$defs"'`.

Command names below are `diverge run …` leaves.

## agents

| Shape | Action | Allows |
|---|---|---|
| Make | `create` | `agents create` |
| Over | `get` | `agents get` |
| Over | `list` | `agents list` (sends what the grant reaches) |
| Over | `edit` | `agents edit`; also needed over the agent when a standalone or connected tool is attached to / detached from that one agent |
| Over | `delete` | `agents delete` |
| Over | `message` | `agents message` (a chat message and a structured decision alike) |
| Over | `logs` | `logs` with `of.type` = `agent` |
| Over | `download` | `agents download` |
| Over | `upload` | `agents upload`, and a transfer landing in the agent |
| Over | `transfer` | `agents transfer` (source side) |
| Over | `filetree` | `agents filetree` |
| Over | `exec` | `agents exec`, and `agents terminal` (a terminal is judged as `exec`) |
| Over | `frontend` | `agents frontend` |
| Over | `connect` | `agents connect …` (MCP exchanges run as the agent) |
| Tagging | `tag`, `untag` | `agents tag`, `agents untag` |

`within` filter members: `names`, `templates`, `creators`, `active` (`true`/`false`, or `"ever"`/`"never"`),
`deployed`, `dependency_tool_templates`, `all_tags`, `any_tags`, `created_from`, `created_to`.

## agents_templates

| Shape | Action | Allows |
|---|---|---|
| Make | `create` | `agents templates create` |
| Over | `get`, `list`, `delete` | the `agents templates` request of the same name |
| Over | `attach` | naming the template in `attach-agent-template` / `detach-agent-template` of a standalone or connected tool (beside the tool's own `attach`/`detach`) |
| Tagging | `tag`, `untag` | `agents templates tag` / `untag` |

`within`: `ids`, `creators`, `in_use`, `all_tags`, `any_tags`, `created_from`, `created_to`.

## standalone_tools

| Shape | Action | Allows |
|---|---|---|
| Make | `create` | `standalone-tools create` |
| Over | `get`, `list`, `edit`, `delete` | the request of the same name |
| Over | `attach` | `attach-agent-instance` (plus `edit` over the agent), `attach-agent-template` (plus `attach` over the template) |
| Over | `detach` | `detach-agent-instance`, `detach-agent-template` (same second grants) |
| Over | `download`, `upload`, `transfer`, `filetree` | the file requests; `upload` also covers a transfer landing in the tool |
| Over | `exec` | `standalone-tools exec` and `terminal` |
| Over | `frontend` | `standalone-tools frontend` |
| Over | `connect` | `standalone-tools connect …`: serving the tool's MCP exchanges to a client, **including another daemon that holds it as a connected tool** |
| Over | `logs` | `logs` with `of.type` = `standalone_tool` |
| Tagging | `tag`, `untag` | `standalone-tools tag` / `untag` |

`within`: `names`, `templates`, `creators`, `modes`, `active`, `deployed`, `attached_agent_instances`,
`attached_agent_templates`, `all_tags`, `any_tags`, `created_from`, `created_to`.

## standalone_tools_templates

| Shape | Action | Allows |
|---|---|---|
| Make | `create` | `standalone-tools templates create` |
| Over | `get`, `list`, `delete` | the request of the same name |
| Tagging | `tag`, `untag` | `standalone-tools templates tag` / `untag` |

`within`: `ids`, `creators`, `in_use`, `all_tags`, `any_tags`, `created_from`, `created_to`.

## connected_standalone_tools

Another daemon's standalone tool, held here under a name.

| Shape | Action | Allows |
|---|---|---|
| Make | `create` | `connected-standalone-tools create` (plus `relay` over the `daemons` record it names) |
| Over | `get`, `list`, `edit`, `delete` | the request of the same name |
| Over | `attach` | `attach-agent-instance` (plus `edit` over the agent), `attach-agent-template` (plus `attach` over the template) |
| Over | `detach` | `detach-agent-instance`, `detach-agent-template` |
| Over | `logs` | `logs` with `of.type` = `connected_standalone_tool` |
| Tagging | `tag`, `untag` | `connected-standalone-tools tag` / `untag` |

`within`: `names`, `daemons` (record IDs), `creators`, `active`, `deployed`, `attached_agent_instances`,
`attached_agent_templates`, `all_tags`, `any_tags`, `created_from`, `created_to`.

## dependency_tools

No making shape: a dependency tool is made by an agent's registration, never by a request.

| Shape | Action | Allows |
|---|---|---|
| Over | `get`, `list` | `dependency-tools get` / `list` |
| Over | `delete` | `dependency-tools delete` (by template: needs the action over every tool of the template) |
| Over | `download`, `upload`, `transfer`, `filetree` | the file requests on a running tool |
| Over | `exec` | `dependency-tools exec` and `terminal` |
| Over | `frontend` | `dependency-tools frontend` |
| Over | `logs` | `logs` with `of.type` = `dependency_tool` |
| Tagging | `tag`, `untag` | `dependency-tools tag` / `untag` |

`within`: `ids`, `owners`, `creators`, `modes`, `scopes`, `depended`, `agents`, `in_use`, `all_tags`,
`any_tags`, `created_from`, `created_to`.

## providers

`within` here is `"any"` or a **provider reach**: `{"identities":[{"issuer":…,"subject":…}],"all_tags":[…],"any_tags":[…]}`,
each member optional. A provider is reached when it is one of `identities` (if any are given) **and** its
record carries every one of `all_tags` **and** any one of `any_tags` (if any are given). So a grant can name
providers by identity, by tag, or both. No other member decodes. The same reach is the `within` of
`providers_daemons` and of `images`.

| Shape | Action | Allows |
|---|---|---|
| Make | `add` | `providers add` |
| Over | `get`, `list` | `providers get` / `list` |
| Over | `edit` | `providers edit` |
| Over | `enable`, `disable` | `providers enable` / `disable` |
| Over | `delete` | `providers delete` |
| Over | `read` | `images read` (whether an image can be supplied, and its manifests, blobs, proof and DIVERGE.md): the daemon asks only the providers the grant reaches |
| Tagging | `tag`, `untag` | `providers tag` / `untag` |

## providers_daemons

What is done *through* providers to the daemons connected to them. Over shape only; `within` is `"any"` or
a provider reach (identities, tags, or both: see `providers` above).

| Action | Allows |
|---|---|
| `list` | `providers daemons list`: only routes through reached providers are listed |
| `listen` | `providers daemons listen`: only messages that came through reached providers |
| `message` | `providers daemons message`: sent through no provider the grant does not reach |

Examples: `{"providers_daemons":{"actions":["list","listen","message"],"within":"any"}}`,
`{"providers_daemons":{"actions":["list"],"within":{"identities":[{"issuer":"…","subject":"github.12345.provider"}]}}}`.

## images

The daemon's Stores: the images it owns on each provider. Over shape only; `within` is `"any"` or a provider
reach (identities, tags, or both: see `providers` above). **A bare array, `{"images":["build"]}`, no longer
decodes.**

| Action | Allows |
|---|---|
| `build` | `images build` on a provider the grant reaches (plus `mount` over the context volume, where one is named) |
| `push` | `images push` to a provider the grant reaches |
| `list` | `images list`: only the Stores on reached providers are listed |

Every action is judged by the provider alone, never by the image's digest. The Store on a provider is the
daemon's: every account whose grant reaches the provider sees and adds to the same Store. `images read`, an
image's DIVERGE.md included, is not held here: it is judged by `read` over providers and `relay` over daemons.

Examples: `{"images":{"actions":["build","push","list"],"within":"any"}}`,
`{"images":{"actions":["build"],"within":{"all_tags":["build"]}}}`.

## daemons

Remote daemons' records.

| Shape | Action | Allows |
|---|---|---|
| Make | `add` | `daemons add` |
| Over | `get`, `list`, `edit`, `delete` | the request of the same name |
| Over | `relay` | **using** the remote daemon, whatever the use: any request whose `relay` starts with the record's ID; naming the record as `daemon` in `connected-standalone-tools create`; naming it as `daemon` in a container's FUSE mount; `images read`, which asks only the remote daemons the grant reaches |
| Tagging | `tag`, `untag` | `daemons tag` / `untag` |

`relay` is the one permission to use a remote daemon. There is no `connect` or `read` over daemons: a role
that still lists either does not decode and must be edited to `relay`. What the remote daemon then allows
the account the record names is the remote's own to judge.

`within`: `ids`, `issuers`, `creators`, `connected`, `all_tags`, `any_tags`, `created_from`, `created_to`.

## accounts

| Shape | Action | Allows |
|---|---|---|
| Make | `create` | `accounts create` (plus `grant` over each role named) |
| Over | `get`, `list`, `delete` | the request of the same name |
| Over | `edit` | `accounts edit` (plus, when the roles change, `grant` over every role in the resulting list) |
| Over | `logs` | `logs` with `of.type` = `account`: the account's command log |
| Over | `disconnect` | `accounts disconnect` |
| Tagging | `tag`, `untag` | `accounts tag` / `untag` |

`within`: `identities`, `roles`, `connected`, `creators`, `all_tags`, `any_tags`, `created_from`,
`created_to`. An administrator with no account is reached only through `identities` and `connected`.
An administrator's command log (`of.type` = `admin`) is read by administrators alone; no grant gives it.

## roles

| Shape | Action | Allows |
|---|---|---|
| Make | `create` | `roles create` |
| Over | `get`, `list`, `edit`, `delete` | the request of the same name |
| Over | `grant` | naming the role in an account's `roles` at `accounts create` / `accounts edit` |
| Tagging | `tag`, `untag` | `roles tag` / `untag` |

`within`: `names`, `accounts`, `creators`, `all_tags`, `any_tags`, `created_from`, `created_to`.

Note the escalation paths: `edit` over a role lets the holder rewrite the grants of a role it may itself
hold; `create` over roles plus `grant` within `"any"` plus `edit` over accounts is full control. Scope
`grant` and `edit` with `within`.

## volumes

| Shape | Action | Allows |
|---|---|---|
| Make | `create` | `volumes create` |
| Over | `get`, `list`, `edit`, `delete`, `stat` | the request of the same name |
| Over | `download`, `upload`, `transfer`, `filetree` | the file requests; `upload` also covers a transfer landing in the volume |
| Over | `mount` | naming the volume in a container's mounts at its create or edit (beside the container's grant) |
| Over | `fuse_serve` | `volumes fuse_serve`: having the volume served over FUSE to the caller. What **another daemon's account** needs here so that its containers can FUSE-mount the volume. `mount` does not imply it |
| Tagging | `tag`, `untag` | `volumes tag` / `untag` |

`within`: `providers`, `names`, `modes`, `mounted`, `all_tags`, `any_tags`, `created_from`, `created_to`.
A grant reaches a volume or a container whole; it cannot narrow by path.

## vault

| Shape | Action | Allows |
|---|---|---|
| Make | `insert` | `vault insert` into a key the vault does not hold yet |
| Over | `insert` | `vault insert` into a key it holds (append or replace a value) |
| Over | `get` | `vault get` (with the value) |
| Over | `list` | `vault list` (never a value) |
| Over | `delete` | `vault delete` |
| Over | `use` | binding the key in a container's `secrets` at its create or edit |
| Tagging | `tag`, `untag` | `vault tag` / `untag` |

`within`: `keys`, `creators`, `all_tags`, `any_tags`, `created_from`, `created_to`.

## postgres, events (bare arrays)

| Kind | Actions | Allows |
|---|---|---|
| `postgres` | `get`, `list` | `postgres get` (which database, no password), `postgres list` (container connections) |
| `events` | `list` | `events list` |

Examples: `{"postgres":["get","list"]}`, `{"events":["list"]}`.

## identities

The identities signed in on the daemon's host.

| Shape | Action | Allows |
|---|---|---|
| Make | `login` | `identities login` of an identity not on the host yet |
| Over | `list` | `identities list` |
| Over | `login` | `identities login` replacing an identity already on the host |
| Tagging | `tag`, `untag` | `identities tag` / `untag` |

`within`: `issuers`, `subjects`, `all_tags`, `any_tags`.

## A read-only "observer" role

```json
{"name":"observer","grants":[
 {"agents":{"actions":["get","list","logs"],"within":"any"}},
 {"agents_templates":{"actions":["get","list"],"within":"any"}},
 {"standalone_tools":{"actions":["get","list","logs"],"within":"any"}},
 {"standalone_tools_templates":{"actions":["get","list"],"within":"any"}},
 {"connected_standalone_tools":{"actions":["get","list","logs"],"within":"any"}},
 {"dependency_tools":{"actions":["get","list","logs"],"within":"any"}},
 {"providers":{"actions":["get","list","read"],"within":"any"}},
 {"providers_daemons":{"actions":["list"],"within":"any"}},
 {"daemons":{"actions":["get","list"],"within":"any"}},
 {"accounts":{"actions":["get","list"],"within":"any"}},
 {"roles":{"actions":["get","list"],"within":"any"}},
 {"volumes":{"actions":["get","list","stat"],"within":"any"}},
 {"vault":{"actions":["list"],"within":"any"}},
 {"postgres":["get","list"]},
 {"events":["list"]},
 {"identities":{"actions":["list"],"within":"any"}}]}
```

## Grants a tool's image declares

A tool container is not an account. It holds the grants its program declared at registration, written in
the same vocabulary with one restriction: `within` is `"any"` or a tags test only (no names), plus, over
agents alone:

- `"attached"`: the agents the tool is attached to.
- `"never_active"`: agents that have never run.
- `{"agents":[{"template":"<agent template id>","index":<n>}, …]}`: exactly the agents listed, and no
  other. The one form that names anything. A tool whose arguments are a list of agents declares that list.

None of the three is valid in a role's grant. See `diverge-tools` and `diverge-images`.
