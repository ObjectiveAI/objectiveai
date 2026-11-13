# `diverge run agents`: every leaf, its body, its answers

Every leaf also has `request-schema` and `response-schema` beneath it (they print JSON Schema and dial
nothing), and takes the global `run` options `--daemon`, `--account`, `--jq`, `--timeout`. Every body may
carry `relay`. `<ref>` is an agent reference: `{"name":…}` or `{"template":…,"index":…}`.

## How to read an answer

- A bare answer is a JSON string: `"created"`, `"not_found"`, `"forbidden"`.
- An answer with a payload is an object with one member: `{"found":{…}}`, `{"error":…}`.
- `"forbidden"` exists on every leaf: no grant reaches what the request names; nothing changed.
- `{"error":…}` exists on every leaf: the daemon (or the container) could not do it, in its own words.
- Answers such as `"not_found"`, `"in_use"`, `"active"` are definite: the daemon looked, nothing changed,
  and retrying the same request unchanged gives the same answer until the stated condition changes.
- A command that prints nothing and exits cleanly got a finish with no answer: an empty directory download,
  or a request the daemon did not serve.

## Templates

| leaf | body | answers |
|---|---|---|
| `agents templates create` | the template: `type:"agent"`, `image`, `memory`, `disk`, `arguments`, optional `secrets`, `description` | `{"created":"<id>"}`, `{"exists":"<id>"}` |
| `agents templates get` | `{"id"}` | `{"found":<listed>}`, `"not_found"` |
| `agents templates list` (`--watch`) | filter + `count` | `{"added":<listed>}`…, `"listed"`, then with `--watch` `{"added"}`, `{"changed"}`, `{"removed"}` |
| `agents templates delete` | `{"id"}` | `"deleted"`, `"not_found"`, `"in_use"` |
| `agents templates tag` | `{"id","tags":[…]}` | `"tagged"`, `"not_found"` |
| `agents templates untag` | `{"id","tags":[…]}` | `"untagged"`, `"not_found"` |

A listed template: `id`, `created`, `creator`, `tags`, `tools` (tools attached to the template), `template`
(the body as handed in). A template made again after a delete keeps its first `created` and `creator`.

Template list filter: `ids`, `creators`, `in_use` (bool: some agent was made from it), `all_tags`,
`any_tags`, `created_from`, `created_to`; plus `count`.

## Agents

| leaf | body | answers |
|---|---|---|
| `agents create` | `template`, optional `name`, `secrets`, `provider`, `fuse_file_mounts`, `fuse_directory_mounts` | `"created"`, `"in_use"`; `{"error":{"kind":"image",…}}` when the template's image has no valid `DIVERGE.md` the daemon can find, or declares neither `chat` nor `structured_decisions` |
| `agents get` | `{"agent":<ref>}` | `{"found":<agent>}`, `"not_found"` |
| `agents list` (`--watch`) | filter + `count` | `{"added":<agent>}`…, `"listed"`, then with `--watch` `{"added"}`, `{"changed"}`, `{"removed"}` |
| `agents edit` | `{"agent":<ref>,"patch":[…]}` | `"edited"`, `"not_found"`, `"active"`, `"in_use"` |
| `agents delete` | `{"agent":<ref>}` | `"deleted"`, `"not_found"`, `"active"` |
| `agents tag` | `{"agent":<ref>,"tags":[…]}` | `"tagged"`, `"not_found"` |
| `agents untag` | `{"agent":<ref>,"tags":[…]}` | `"untagged"`, `"not_found"` |
| `agents message` | `{"agent":<ref>,"type":"chat","content":[<block>…],"watch"?:true}` or `{"agent":<ref>,"type":"structured_decisions","content":[<block>…],"questions":[<question>…],"watch"?:true}` | `"delivered"`, `"no_chat"`, `"no_structured_decisions"`, `{"unsupported_modalities":{"prompt":[…],"instruction":[…],"choice":[…]}}`; invalid questions are `{"error":{"kind":"questions",…}}`; watching: `{"item":{"agent":{…}}}` lines, or `"dropped"` (no `"not_found"`: an unknown agent is `{"error":…}`) |

A listed agent:

| member | meaning |
|---|---|
| `name` | absent for an unnamed agent |
| `template`, `index` | the once-and-for-all reference |
| `creator` | `{"type":"client","identity":{…}}`, `{"type":"agent",…}` or `{"type":"tool",…}`: the direct maker |
| `created` | RFC 3339 UTC |
| `active`, `last_active` | working now; when that last changed (absent if it never ran a loop) |
| `deployed`, `last_deployed` | container up now; when that last changed |
| `provider` | where it is or was last deployed: `{"identity":{"issuer","subject"}}` |
| `logs_index` | index of its latest log item as of its last change of state; `0` for an empty log |
| `tools` | attached standalone tools (`{"record":{template,index,name}}`) and connected tools (`{"connected_standalone":{…}}`), direct attachments first, then its template's |
| `dependencies` | dependency template ids its last registration declared; absent when none or never run |
| `tags` | sorted; absent when empty |

Agent list filter: `names`, `templates`, `creators`, `active` (`true` | `false` | `"ever"` | `"never"`),
`deployed`, `dependency_tool_templates`, `all_tags`, `any_tags`, `created_from`, `created_to`; plus `count`.
Members that list candidates match any one of them; `all_tags` needs every tag; an empty list is absent.
The list is oldest created first. `logs_index` on a list item does not move with every log line: watch the
log itself (`diverge-logs`) to follow a conversation.

## MCP as the agent

Body for all four: `{"agent":<ref>}`. The connect answers `"not_found"`, `"forbidden"` or `{"error":…}` if it
cannot be opened; otherwise the line printed is the MCP result.

| leaf | flags | prints |
|---|---|---|
| `agents connect list-tools` | `--cursor <CURSOR>` | one page of the agent's merged MCP tools |
| `agents connect call-tool` | `--name <NAME>` (required), `--arguments <JSON object>` | the MCP call result |
| `agents connect list-resources` | `--cursor <CURSOR>` | one page of the agent's MCP resources |
| `agents connect read-resource` | `--uri <URI>` (required) | the resource's contents |

## Files

| leaf | flags | body | answers |
|---|---|---|---|
| `agents filetree` | `--watch` | `{"agent":<ref>}` | `{"filetree":{"type":"snapshot",…}}`, then changes with `--watch`; `"not_found"` |
| `agents download` | `--path <LOCAL>` (required) | `{"agent":<ref>,"path":[…]}` | one line per file written; `"not_found"` |
| `agents upload` | `--path <LOCAL>` (required) | `{"kind":"file","agent":<ref>,"path":[…]}` or `{"kind":"directory","agent":<ref>,"path":[…],"files":["a","b/c"]}` | `"uploaded"`, `"not_found"` |
| `agents transfer` | | `{"agent":<ref>,"path":[…],"to":<destination>}` | `"transferred"`, `"not_found"`, `"no_destination"`, `"held"` |

On a failure part-way, files that already landed stay; each file is whole or as it was, never half.

## Running things inside

| leaf | flags | body | answers |
|---|---|---|---|
| `agents exec` | `--raw` | `{"agent":<ref>,"argv":[…],"cwd":[…]?,"env":{…}?}` | `{"stdout":"<base64>"}`, `{"stderr":"<base64>"}` any number, then `{"exited":<code>}` or `{"signaled":<n>}`; or `"not_found"` |
| `agents terminal` | | `{"agent":<ref>,"cols":<n>,"rows":<n>}` | the shell's output raw on stdout; `"not_found"`, `"forbidden"`, `{"error":…}` as a JSON line on stderr; exit code is the shell's |
| `agents frontend` | | `{"agent":<ref>,"method":"GET","path":"/frontend/…","headers":[["Accept","text/html"]]}`, request body on stdin | response head as one JSON line on stderr, body raw on stdout; `"not_found"`, `"no_frontend"`, `"forbidden"`, `{"error":…}` on stderr |

## What starts or holds the container

| operation | container not running |
|---|---|
| `message`, `connect` exchanges | started, with its dependency tools; stopped by the idle clock after `idle_seconds` unused |
| `exec`, `terminal`, `frontend` | started for it, held up while it runs |
| `filetree`, `download`, `upload`, `transfer` | started for the operation, stopped when it ends |
| `create`, `get`, `list`, `edit`, `tag`, `untag` | nothing is started |
