# `standalone-tools` and `dependency-tools`: every leaf, its body, its answers

Every leaf has `request-schema` and `response-schema` beneath it and takes the global `run` options
`--daemon`, `--account`, `--jq`, `--timeout`. Every body may carry `relay`. Every leaf can also answer
`"forbidden"` (no grant reaches what is named; nothing changed) and `{"error":…}`; they are left out of the
tables. Bare answers are JSON strings; answers with a payload are one-member objects.

`<tool>` is `{"name":…}` or `{"template":…,"index":…}`. `<agent>` is the same two forms over agents.
`<dep>` is `{"template":"<dependency template id>","owner":<owner>}` with owner `{"agent":<agent>}`,
`{"agent_template":{"template":…}}` or `"global"`.

## `diverge run standalone-tools templates`

| leaf | body | answers |
|---|---|---|
| `create` | `type:"tool"`, optional `mode`, `image`, `memory`, `disk`, `arguments`, optional `secrets`, `description` | `{"created":"<id>"}`, `{"exists":"<id>"}` |
| `get` | `{"id"}` | `{"found":{id, created, creator, tags, template}}`, `"not_found"` |
| `list` (`--watch`) | `ids`, `creators`, `in_use`, `all_tags`, `any_tags`, `created_from`, `created_to`, `count` | `{"added":…}`…, `"listed"`; with `--watch` also `{"changed":…}`, `{"removed":…}` |
| `delete` | `{"id"}` | `"deleted"`, `"not_found"`, `"in_use"` |
| `tag` | `{"id","tags":[…]}` | `"tagged"`, `"not_found"` |
| `untag` | `{"id","tags":[…]}` | `"untagged"`, `"not_found"` |

## `diverge run standalone-tools`

| leaf | flags | body | answers |
|---|---|---|---|
| `create` | | `template`, optional `name`, `secrets`, `provider`, `fuse_file_mounts`, `fuse_directory_mounts` | `"created"`, `"in_use"` |
| `get` | | `{"tool":<tool>}` | `{"found":<item>}`, `"not_found"` |
| `list` | `--watch` | filter + `count` | `{"added":<item>}`…, `"listed"`; with `--watch` also `{"changed"}`, `{"removed"}` |
| `edit` | | `{"tool":<tool>,"patch":[…]}` | `"edited"`, `"not_found"`, `"active"`, `"in_use"` |
| `delete` | | `{"tool":<tool>}` | `"deleted"`, `"not_found"`, `"attached"` |
| `tag` | | `{"tool":<tool>,"tags":[…]}` | `"tagged"`, `"not_found"` |
| `untag` | | `{"tool":<tool>,"tags":[…]}` | `"untagged"`, `"not_found"` |
| `attach-agent-instance` | | `{"tool":<tool>,"agent":<agent>}` | `"attached"`, `"no_tool"`, `"no_agent"` |
| `detach-agent-instance` | | `{"tool":<tool>,"agent":<agent>}` | `"detached"`, `"no_tool"`, `"no_agent"` |
| `attach-agent-template` | | `{"tool":<tool>,"template":"<agent template id>"}` | `"attached"`, `"no_tool"`, `"no_template"` |
| `detach-agent-template` | | `{"tool":<tool>,"template":"<agent template id>"}` | `"detached"`, `"no_tool"`, `"no_template"` |
| `connect list-tools` | `--cursor` | `{"tool":<tool>}` | the MCP result; or `"not_found"` |
| `connect call-tool` | `--name` (required), `--arguments <JSON object>` | `{"tool":<tool>}` | the MCP result; or `"not_found"` |
| `connect list-resources` | `--cursor` | `{"tool":<tool>}` | the MCP result; or `"not_found"` |
| `connect read-resource` | `--uri` (required) | `{"tool":<tool>}` | the MCP result; or `"not_found"` |
| `filetree` | `--watch` | `{"tool":<tool>}` | `{"filetree":{"type":"snapshot",…}}`, changes with `--watch`; `"not_found"` |
| `download` | `--path <LOCAL>` (required) | `{"tool":<tool>,"path":[…]}` | one line per file written; `"not_found"` |
| `upload` | `--path <LOCAL>` (required) | `{"kind":"file","tool":<tool>,"path":[…]}` or `{"kind":"directory","tool":<tool>,"path":[…],"files":[…]}` | `"uploaded"`, `"not_found"` |
| `transfer` | | `{"tool":<tool>,"path":[…],"to":<destination>}` | `"transferred"`, `"not_found"`, `"no_destination"`, `"held"` |
| `exec` | `--raw` | `{"tool":<tool>,"argv":[…],"cwd":[…]?,"env":{…}?}` | `{"stdout":"<base64>"}`, `{"stderr":"<base64>"}`…, then `{"exited":<code>}` or `{"signaled":<n>}`; `"not_found"` |
| `terminal` | | `{"tool":<tool>,"cols":<n>,"rows":<n>}` | shell output raw on stdout; other answers as a JSON line on stderr |
| `frontend` | | `{"tool":<tool>,"method":…,"path":"/frontend/…","headers":[[name,value]…]}`, body on stdin | head as a JSON line on stderr, body raw on stdout; `"not_found"`, `"no_frontend"` on stderr |

A standalone tool's list item: `name`, `template`, `index`, `mode`, `provider` (where it runs or last ran),
`id` (container id while running), `creator`, `created`, `active`, `last_active`, `deployed`,
`last_deployed`, `attached_agent_instances` (each `{template,index,name}`), `attached_agent_templates`
(ids), `permissions`, `tags`.

List filter: `names`, `templates`, `creators`, `modes`, `active` (bool), `deployed` (bool),
`attached_agent_instances` (agent names; attached to every one), `attached_agent_templates` (ids; attached
to every one), `all_tags`, `any_tags`, `created_from`, `created_to`; plus `count`.

Transfer `<destination>`: `{"agent":<agent>,"path":[…]}`, `{"tool":<tool>,"path":[…]}`,
`{"dependency":<dep>,"path":[…]}`, `{"volume":{"provider":{"issuer","subject"},"name":…},"path":[…]}`. The
path has at least one component. A file source lands at the path; a directory source lands as the path, and
files already there that the source lacks stay.

## `diverge run dependency-tools`

| leaf | flags | body | answers |
|---|---|---|---|
| `get` | | `{"dependency":<dep>}` | `{"found":<item>}`, `"not_found"` |
| `list` | `--watch` | filter + `count` | `{"added":<item>}`…, `"listed"`; with `--watch` also `{"changed"}`, `{"removed"}` |
| `delete` | | `{"id":"<dependency template id>"}` | `"deleted"`, `"not_found"`, `"in_use"` |
| `tag` | | `{"dependency":<dep>,"tags":[…]}` | `"tagged"`, `"not_found"` |
| `untag` | | `{"dependency":<dep>,"tags":[…]}` | `"untagged"`, `"not_found"` |
| `filetree` | `--watch` | `{"dependency":<dep>}` | as standalone; `"not_found"` also when it does not run |
| `download` | `--path <LOCAL>` (required) | `{"dependency":<dep>,"path":[…]}` | as standalone; `"not_found"` also when it does not run |
| `upload` | `--path <LOCAL>` (required) | `{"kind":"file"|"directory","dependency":<dep>,"path":[…],…}` | `"uploaded"`, `"not_found"` (also when it does not run) |
| `transfer` | | `{"dependency":<dep>,"path":[…],"to":<destination>}` | `"transferred"`, `"not_found"`, `"no_destination"`, `"held"` |
| `exec` | `--raw` | `{"dependency":<dep>,"argv":[…],…}` | as standalone; `"not_found"` when it does not run |
| `terminal` | | `{"dependency":<dep>,"cols":<n>,"rows":<n>}` | as standalone; `"not_found"` when it does not run |
| `frontend` | | `{"dependency":<dep>,"method":…,"path":…,"headers":[…]}` | as standalone; `"not_found"` when it does not run |

There is no `create`, `edit`, `attach`, `detach` or `connect` in this family: registrations make and attach
dependency tools, and agents reach them through their own tool list.

## What starts or holds a tool's container

| trigger | transient standalone | service standalone | dependency |
|---|---|---|---|
| `create` | nothing | nothing | (no create) |
| attach to an agent or template | nothing | started and kept up | (declared by registration: deployed at the agent's start) |
| attached agent begins a loop | started | already up | already up |
| `connect` exchange | started, held while open | held while open | (no connect) |
| `exec`, `terminal`, `frontend` | started, held while running | already up or started | must already run |
| `filetree`, `download`, `upload`, `transfer` | started, stopped after | used as it runs, or started and stopped after | must already run |
| nothing using it for `idle_seconds` | stopped | retired only once nothing is attached and no connect is open | transient: stopped with its agent; service: retired once no agent is attached |

## Grants by leaf

| leaf | grant |
|---|---|
| `standalone-tools templates create` / `get` / `list` / `delete` / `tag` / `untag` | the same action over `standalone_tools_templates` |
| `standalone-tools create` | `create` over `standalone_tools`; plus `mount` over each volume named and `use` over each vault key named |
| `get`, `list`, `edit`, `delete`, `tag`, `untag`, `download`, `upload`, `transfer`, `filetree`, `frontend`, `connect` | the same action over `standalone_tools` |
| `exec`, `terminal` | `exec` |
| `attach-agent-instance`, `detach-agent-instance` | `attach` / `detach` over the tool and `edit` over the agent |
| `attach-agent-template`, `detach-agent-template` | `attach` / `detach` over the tool and `attach` over the agent template |
| reading a tool's log | `logs` over the tool |
| `dependency-tools get`, `list`, `delete`, `tag`, `untag`, `download`, `upload`, `transfer`, `filetree`, `frontend` | the same action over `dependency_tools` (there is no making action: registrations make them) |
| `dependency-tools exec`, `terminal` | `exec` over `dependency_tools` |
| a transfer's destination | `upload` over the agent, tool, dependency tool or volume it lands in |

Grant shapes, `within`, tags and diagnosing `"forbidden"` are in `diverge-access`.
