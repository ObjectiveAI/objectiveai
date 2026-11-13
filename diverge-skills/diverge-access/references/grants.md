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
| Make | `create` | `connected-standalone-tools create` (plus `connect` over the `daemons` record it names) |
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

`within` here is **only** `"any"` or a tags test `{"all_tags":[…],"any_tags":[…]}` over the provider
record's tags (the providers list filter has no tags, so the grant uses the tags test directly; no other
member decodes).

| Shape | Action | Allows |
|---|---|---|
| Make | `add` | `providers add` |
| Over | `get`, `list` | `providers get` / `list` |
| Over | `edit` | `providers edit` |
| Over | `enable`, `disable` | `providers enable` / `disable` |
| Over | `delete` | `providers delete` |
| Over | `oci` | `images oci`: the daemon asks only the providers the grant reaches |
| Tagging | `tag`, `untag` | `providers tag` / `untag` |

## providers_daemons

What is done *through* providers to the daemons connected to them. Over shape only; `within` is `"any"` or
a tags test over the provider records' tags.

| Action | Allows |
|---|---|
| `list` | `providers daemons list`: only routes through reached providers are listed |
| `listen` | `providers daemons listen`: only messages that came through reached providers |
| `message` | `providers daemons message`: sent through no provider the grant does not reach |

Example: `{"providers_daemons":{"actions":["list","listen","message"],"within":"any"}}`.

## daemons

Remote daemons' records.

| Shape | Action | Allows |
|---|---|---|
| Make | `add` | `daemons add` |
| Over | `get`, `list`, `edit`, `delete` | the request of the same name |
| Over | `connect` | naming the record as `daemon` in `connected-standalone-tools create` |
| Over | `relay` | any request whose `relay` starts with the record's ID |
| Over | `oci` | `images oci`: the daemon asks only the remote daemons the grant reaches |
| Tagging | `tag`, `untag` | `daemons tag` / `untag` |

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

## postgres, events, images (bare arrays)

| Kind | Actions | Allows |
|---|---|---|
| `postgres` | `get`, `list` | `postgres get` (which database, no password), `postgres list` (container connections) |
| `events` | `list` | `events list` |
| `images` | `info` | `images info` (an image's DIVERGE.md). `images oci` is judged by `oci` over providers and over daemons instead |

Examples: `{"postgres":["get","list"]}`, `{"events":["list"]}`, `{"images":["info"]}`.

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
 {"providers":{"actions":["get","list"],"within":"any"}},
 {"providers_daemons":{"actions":["list"],"within":"any"}},
 {"daemons":{"actions":["get","list"],"within":"any"}},
 {"accounts":{"actions":["get","list"],"within":"any"}},
 {"roles":{"actions":["get","list"],"within":"any"}},
 {"volumes":{"actions":["get","list","stat"],"within":"any"}},
 {"vault":{"actions":["list"],"within":"any"}},
 {"postgres":["get","list"]},
 {"events":["list"]},
 {"images":["info"]},
 {"identities":{"actions":["list"],"within":"any"}}]}
```

## Grants a tool's image declares

A tool container is not an account. It holds the grants its program declared at registration, written in
the same vocabulary with one restriction: `within` is `"any"` or a tags test only (no names), plus, over
agents alone, the words `"attached"` (the agents the tool is attached to) and `"never_active"` (agents that
have never run). Those two words are not valid in a role's grant. See `diverge-tools` and `diverge-images`.
