---
name: diverge-agents
description: Diverge agents through the `diverge run agents` commands — agent templates (`agents templates create|get|list|delete|tag|untag`; the template body with image, memory, disk, arguments, secrets; the template id as a hash), agents (`agents create|get|list|edit|delete|tag|untag`, provider pin, volume mounts, FUSE mounts, secret bindings), sending an agent a chat message (`agents message chat`, content blocks, delivery order, idle stop and restart), putting a structured decision to an agent (`agents message structured-decisions`: a prompt and typed questions — noul, choice, score — answered with numbers in the log), MCP as an agent (`agents connect list-tools|call-tool|list-resources|read-resource`), files in an agent's container (`agents filetree|download|upload|transfer`), `agents exec`, `agents terminal`, `agents frontend`, agent references (`{"name":…}` vs `{"template":…,"index":…}`), and the agent lifecycle its log shows (deployed, active, inactive, undeployed). Load it when asked to make, run, message, inspect, change, debug or delete a Diverge agent or agent template.
---

# Diverge agents

An **agent** is one container the daemon runs on a provider: an agent-loop program (a model harness, a
script, or a person at a frontend) that takes messages and answers them, calling the MCP tools the daemon
serves it. An agent is made from an **agent template** (image, limits, arguments, secret names), which is
held by its hash, plus what is the agent's own: a name, a provider, mounts, and which vault key each secret
is. The daemon keeps the agent's record and its whole conversation (its log); the container itself only runs
while there is work for it.

Everything here is one request per command: `diverge run agents <leaf> '<JSON body>'`. Every answer is one
compact JSON line. Run `<leaf> request-schema` or `<leaf> response-schema` for exact shapes; neither dials a
daemon. Every body may also carry `relay` (see `diverge-daemons`).

```bash
export PATH="$HOME/.diverge/bin:$PATH"
diverge run agents --help
diverge run agents create request-schema
```

## Order of operations

1. An image is in the provider's store and you know its digest (`diverge-images`).
2. Any secret the image needs is in the vault (`diverge-storage`).
3. `agents templates create` → the template id.
4. `agents create` → the agent (nothing runs yet).
5. Attach tools if wanted (`diverge-tools`).
6. `agents message chat` → the container starts, the agent works, the container is stopped when idle.
7. Read what it said with `diverge run logs` (`diverge-logs`).

## Naming an agent: two references

Every request that acts on an agent takes `"agent": <reference>`, and you choose the form each time:

- `{"name":"echo"}` — the name its create gave it. Names are optional, unique among the caller's agents, and
  free again after a delete. An agent made without a name cannot be reached this way.
- `{"template":"<template id>","index":3}` — the template it was made from and its number among all agents
  ever made from that template (first is `1`, never reused, deleted ones included). This names the agent once
  and for all and always works. `agents list` reports both members on every item.

An object with members of both forms is malformed. A template is always named by its `id` alone.

## Agent templates

A template is everything every agent made from it shares. It names no provider, no mounts and no name, so it
can be shared between daemons and hashes the same anywhere.

```bash
diverge run agents templates create '{
  "type": "agent",
  "description": "Echo agent. Needs no volumes.",
  "image": {"digest": "sha256:b031dd…", "references": [{"registry": "docker.io", "name": "diverge-local/python"}]},
  "memory": 536870912,
  "disk": 1073741824,
  "arguments": {"python": "…script source…"},
  "secrets": ["OPENROUTER_API_KEY"]
}'
```

| member | meaning |
|---|---|
| `type` | `"agent"`. Required; a tool template (`"type":"tool"`) is a different namespace and hashes apart. |
| `image` | `{"digest":"<algorithm>:<hex>","references":[{"registry":…,"name":…}]}`. The digest is the image. References only say where the bytes may be fetched and are not in the hash. The daemon refuses to start a run unless a reference names a registry the provider lists, so include one even for a locally built image. |
| `memory` | Bytes. A hard ceiling: a process over it is killed. |
| `disk` | Bytes the container may write over its image. Mounted volumes are not counted. |
| `arguments` | Any JSON value, handed to the image's program verbatim. Its shape is the image's own: read the image's `DIVERGE.md` (`diverge run images info`, see `diverge-images`). |
| `secrets` | Names only, e.g. `["OPENROUTER_API_KEY"]`. Each agent binds every name to a vault key at create. Optional. |
| `description` | Free text for whoever creates agents from it (which mounts it needs, and so on). In the hash. Optional. |

Answers: `{"created":"<id>"}`, `{"exists":"<id>"}` (the same template was made before; not a failure),
`"forbidden"`, `{"error":…}`. The image is not checked here; `agents create` checks it. The **id** is the lowercase hex SHA-256 of the template's canonical JSON
(64 characters). Capture it:

```bash
TPL="$(diverge run agents templates create --jq '.created // .exists' "$(cat template.json)" | tr -d '"')"
```

A template cannot be edited: a changed body is a new template with a new id.

**Dependencies are not a template member.** The tools an agent depends on are declared by the agent's
*program* when its container registers. For the agent-loop images in this repository that declaration is
copied from `arguments.dependencies`, a list of dependency tool templates (image, memory, disk, arguments,
secrets, `"mode":"transient"` with optional mounts of the agent's own paths, or `"mode":"service"` with a
`scope` of `instance`, `template` or `global`). The daemon deploys them when the agent's container starts.
See `diverge-tools` for the full shape and what each mode and scope means.

Other template leaves:

| command | body | answers |
|---|---|---|
| `agents templates get` | `{"id":"<id>"}` | `{"found":{id, created, creator, tags, tools, template}}`, `"not_found"` |
| `agents templates list` | `{}` or a filter: `ids`, `creators`, `in_use`, `all_tags`, `any_tags`, `created_from`, `created_to`, `count` | one `{"added":…}` per template, then `"listed"` |
| `agents templates delete` | `{"id":"<id>"}` | `"deleted"`, `"not_found"`, `"in_use"` (an agent was made from it: delete those agents first) |
| `agents templates tag` | `{"id":"<id>","tags":["crew"]}` | `"tagged"`, `"not_found"` |
| `agents templates untag` | `{"id":"<id>","tags":["crew"]}` | `"untagged"`, `"not_found"` |

A listed template's `tools` are the tools attached to the template (every agent made from it has them).
Tags are the caller's, not in the hash. Deleting a template removes its tool attachments with it.

## Creating an agent

```bash
# minimal: runs on whichever provider the daemon chooses, mounts nothing
diverge run agents create "{\"template\":\"$TPL\",\"name\":\"echo\"}"

# fuller: secrets bound, pinned to a provider, one volume mounted
diverge run agents create '{
  "template": "<id>",
  "name": "researcher",
  "secrets": {"OPENROUTER_API_KEY": "or-key"},
  "provider": {
    "identity": {"issuer": "https://identity.diverge.network", "subject": "github.12345.provider"},
    "volume_mounts": [
      {"volume_name": "notes", "volume_relative_path": [], "volume_mode": "persistent", "container_path": ["data"]}
    ]
  }
}'
```

- `template` (required): an agent template id of the caller's. A tool template's id is an error.
- `name` (optional): unique among the caller's agents; taken → `"in_use"`.
- `secrets`: every name the template declares, and no other, each mapped to a vault key that exists with at
  least one value. Naming a key needs the `use` grant over it. Wrong or missing names are the create's error.
- `provider`: pins the agent to one provider (by `{"issuer","subject"}`) and mounts that provider's volumes.
  Absent, the daemon picks a provider at each start and the agent mounts no volume.
- `fuse_file_mounts`, `fuse_directory_mounts`: files or directories of any provider's volume, served live
  through the daemon.

Mount shapes, rules and edit patches are in [references/mounts.md](references/mounts.md). There is no
environment, entrypoint or port to set: the mounts and the template's arguments are the only provisioning.

Answers: `"created"`, `"in_use"`, `"forbidden"`, `{"error":…}`. **A create starts nothing.** The agent has no
account and asks the daemon nothing itself; everything it reaches, it reaches through the tools it is served.

## Getting and listing

```bash
diverge run agents get '{"agent":{"name":"echo"}}'            # {"found":{…}} or "not_found"
diverge run agents list                                        # every agent, then "listed"
diverge run agents list '{"active":true,"all_tags":["crew"]}'
diverge run agents list --watch '{"templates":["<id>"]}'       # stays open: added / changed / removed
```

A list prints `{"added":{…}}` per agent, oldest first, then `"listed"`, and ends there unless `--watch`.
An item carries `name`, `template`, `index`, `creator`, `created`, `active`, `last_active`, `deployed`,
`last_deployed`, `provider`, `logs_index` (length of its log), `tools` (attached standalone and connected
tools), `dependencies` (dependency template ids its last registration declared) and `tags`.

Filter members (all optional; lists match any one of, except `all_tags`): `names`, `templates`, `creators`,
`active` (`true`, `false`, `"ever"`, `"never"`), `deployed`, `dependency_tool_templates`, `all_tags`,
`any_tags`, `created_from`, `created_to`, and `count` (keep to the first N that match).

`active` means a loop is running or an MCP exchange is in flight; `deployed` means the container is up. An
agent can be deployed and inactive (idling before it is stopped).

## Editing

An edit is a JSON Patch (RFC 6902) applied to the agent's editable document: `name`, `volume_mounts`,
`fuse_file_mounts`, `fuse_directory_mounts`, `secrets`. Applied whole or not at all.

```bash
diverge run agents edit '{"agent":{"name":"echo"},"patch":[{"op":"replace","path":"/name","value":"echo-2"}]}'
diverge run agents edit '{"agent":{"name":"x"},"patch":[{"op":"replace","path":"/secrets/OPENROUTER_API_KEY","value":"or-key-2"}]}'
```

Answers: `"edited"`, `"not_found"`, `"in_use"` (the new name is another agent's), `"active"`, `"forbidden"`,
`{"error":…}`.

- While the container is **running**, the only change accepted is FUSE mounts appended to either list
  (`"path":"/fuse_file_mounts/-"`); they are mounted into the running container. Anything else answers
  `"active"`: wait until the agent is undeployed (about `idle_seconds` after its last use) and ask again.
- The template, image, limits, arguments and pinned provider never change. For different arguments, make a
  new template and a new agent.
- An agent pinned to no provider cannot be given `volume_mounts`.
- New secret bindings are used from the container's next activation.

## Deleting, tagging

```bash
diverge run agents delete '{"agent":{"name":"echo"}}'   # "deleted" | "not_found" | "active"
diverge run agents tag    '{"agent":{"name":"echo"},"tags":["crew","blue"]}'   # "tagged"
diverge run agents untag  '{"agent":{"name":"echo"},"tags":["blue"]}'          # "untagged"
```

- Delete is irreversible: the container is stopped, the record goes, the name is free. Its
  `{"template","index"}` is never given to another agent. An agent with a loop running answers `"active"`
  and is left alone; wait and retry. Its instance-owned dependency tools go with it.
- Tags are plain strings. Tagging an existing tag, or untagging an absent one, is not a failure. Tags are
  what grants and filters select by (`diverge-access`).

## Messaging

An agent is asked one of two things, each a leaf of `agents message`, both under the one `message` grant:

- `agents message chat`: a message that joins the agent's conversation. Everything below, up to
  [Structured decisions](#structured-decisions), is about chat.
- `agents message structured-decisions`: a prompt and typed questions, answered with numbers.

```bash
diverge run agents message chat '{"agent":{"name":"echo"},"content":[{"type":"text","text":"hello"}]}'
# "delivered"
```

`content` is an ordered list of MCP content blocks, relayed to the agent's program verbatim:

| block | shape |
|---|---|
| text | `{"type":"text","text":"…"}` |
| image | `{"type":"image","data":"<base64>","mimeType":"image/png"}` |
| audio | `{"type":"audio","data":"<base64>","mimeType":"audio/wav"}` |
| resource | `{"type":"resource","resource":{"uri":"file:///a.txt","mimeType":"text/plain","text":"…"}}` (or `"blob":"<base64>"` in place of `text`) |
| resource link | `{"type":"resource_link","uri":"https://…","name":"…"}` |

Rules:

- **One request, one answer**: `"delivered"`, `"forbidden"`, `"no_chat"`,
  `{"unsupported_modalities":[…]}` or `{"error":…}`. `delivered` means the agent's
  program accepted the message (the loop it started took it, or it went onto the program's own queue). It
  does not mean the agent has answered. The answer is in the log.
- **Nothing takes a message back.** There is no cancel and no dequeue.
- **The first message starts the container** (and every dependency tool the agent declares). The request
  waits for that, however long it takes; use `--timeout` if you need a bound. An abandoned request does not
  recall the message.
- **Order**: messages to one agent are delivered in the order one connection sent them. Each CLI command is
  its own connection, so run commands one after another when order matters.
- **Joining**: messages that are waiting while the agent starts are joined, in order, into its first turn as
  one user message. Text parts stay separate blocks.
- **Idle stop**: once the agent is inactive and unused for the daemon's `idle_seconds` (default 10), its
  container is stopped. The record, the conversation and anything on mounted volumes persist; the next
  message starts it again. Files written only to the container's own disk are gone with the container.
- **What the agent takes**: the daemon holds the message to the `chat` of the agent's image's `DIVERGE.md`
  before sending anything. `"no_chat"`: the image does not chat, and no message is taken.
  `{"unsupported_modalities":["audio"]}`: the message carries an image, audio or a file (a resource with
  `blob` that is neither `image/*` nor `audio/*`) that the image's `prompt_modalities` says `false` to; the
  whole message is refused. Either way nothing was started and the agent's log has no line of it. Text, a
  text resource and a resource link are always taken.
- **Errors**: no such agent is an `{"error":…}` here (not `"not_found"`); so is an image that refuses a block
  kind it cannot take (the whole message is refused), and a run that could not start (missing secret value,
  a dependency that did not deploy, no capable provider). The same failure is kept in the agent's log.
- A first message that fails with "connection ended before the channel answered" usually means dependency
  tools were still deploying. Send it again.

### Structured decisions

```bash
diverge run agents message structured-decisions '{
  "agent": {"name": "judge"},
  "content": [{"type":"text","text":"Review this pull request."}],
  "questions": [
    {"id":"risk","type":"noul","instructions":[{"type":"text","text":"How risky is merging it, from 0 to 1?"}]},
    {"id":"verdict","type":"choice","instructions":[{"type":"text","text":"What should happen?"}],
     "options":[{"value":"merge"},{"value":"revise","description":[{"type":"text","text":"Send it back with comments."}]}]},
    {"id":"quality","type":"score","instructions":[{"type":"text","text":"Rate the code."}],
     "levels":[[{"type":"text","text":"Poor"}],[{"type":"text","text":"Fine"}],[{"type":"text","text":"Excellent"}]]}
  ]
}'
# "delivered"
```

`content` is the prompt, the same blocks as chat. Each question has an `id` (the key its answer comes back
under), a `type`, and `instructions` (content blocks):

| `type` | adds | answered with |
|---|---|---|
| `noul` | nothing | `{"noul": 0.25}` |
| `choice` | `options`: each `{"value": "<text>", "description"?: [blocks]}` | `{"probabilities": {"<value>": 0.7, …}}` |
| `score` | `levels`: each a list of blocks | `{"score": 3.4, "probabilities": [0.1, 0.2, …]}` |

Rules:

- **The answer is the decision's fate, not the agent's answers**: `"delivered"`, `"forbidden"`,
  `"no_structured_decisions"`, `{"unsupported_modalities":{"prompt":[…],"choice":[…]}}` or `{"error":…}`.
  The agent's answers are in its log: see `diverge-logs`.
- **The image must declare them.** `"no_structured_decisions"` when the image's `DIVERGE.md` says
  `structured_decisions: false` (every image in this repository does, so nothing here answers one yet).
- **The questions are validated before anything is sent**; a fault is
  `{"error":{"kind":"questions","question":"<id>","error":"…"}}`:
  - at least one question; ids non-empty and distinct;
  - `instructions` non-empty;
  - a choice has at least two options, each `value` non-empty and distinct within the question, each
    `description` absent or non-empty;
  - a score has at least two levels, each non-empty.
  - "Non-empty" means at least one block that is not an empty text block. Whitespace counts and nothing
    is trimmed; any image, audio, resource or link counts.
- **Modalities**: `content` and every `instructions` are held to the image's
  `structured_decisions.prompt_modalities`; every option `description` and every score level to its
  `choice_modalities`. The answer lists what was carried and not taken, by where.
- **Order**: chat messages and decisions to one agent are taken in the order they were sent, whichever
  each is. A decision is not queued onto a running turn: it waits until the agent has finished everything
  before it and is idle, runs alone, and finishes before anything after it starts. So `"delivered"` for a
  decision can come long after it was sent, and a chat sent after a decision waits for the decision to
  finish.
- Nothing takes a decision back.

Read the reply:

```bash
diverge run logs '{"of":{"type":"agent","name":"echo"}}'
diverge run logs '{"of":{"type":"agent","name":"echo"},"watch":true,"type":"assistant_text_content"}'
```

## The lifecycle, as the log shows it

Only the daemon writes these four items, in this order, per run of the container:

```
deployed            container is up on a provider (names the provider)
  active            a loop began, or an MCP exchange is in flight
    user_text_content / assistant_* / tool_response / usage …
  inactive          the work ceased; the idle clock runs from here
  active … inactive (again, for each further stretch of work)
undeployed          container is down: {"interrupted":false} and no "reason" when it was asked to stop
```

- `undeployed` with a `reason` is an unexpected end: `provider_lost`, `provider_disabled`,
  `provider_edited`, `daemon_stopped`, `exited` (the container exited by itself) or `failed` (an `error`
  item just before says how). `interrupted: true` means it was working when it went down.
- A run that never came up keeps an `error` item and no `deployed`.
- Each user part carries `sender` (who sent the message) and `key` (which message it belongs to).

`diverge-logs` covers the log read in full (spans, `type`, `count`, `watch`).

## MCP as an agent: `agents connect`

Runs one MCP exchange exactly as the agent's own container would: the same merged tool list, the same names,
logged in each tool's log as the agent's call. Use it to see what an agent can call, or to drive a tool by
hand as that agent. Body is `{"agent":<reference>}`; the exchange is the subcommand.

```bash
diverge run agents connect list-tools '{"agent":{"name":"echo"}}'
diverge run agents connect call-tool --name notebook_remember --arguments '{"text":"hi"}' '{"agent":{"name":"echo"}}'
diverge run agents connect list-resources '{"agent":{"name":"echo"}}'
diverge run agents connect read-resource --uri 'memo://1' '{"agent":{"name":"echo"}}'
```

- Output is the MCP result's own JSON. `list-tools` and `list-resources` print one page; pass the result's
  `nextCursor` as `--cursor` for the next.
- Tool names are `<prefix>_<tool name>`, the prefix naming which attached tool serves it (see
  `diverge-tools`). Each listed tool also says who serves it under `_meta` → `diverge.network`.
- An exchange starts the agent's container and its tools if they are down, and keeps the agent `active`
  while in flight. A container that will not start is that exchange's error.
- The connect itself answers `"not_found"`, `"forbidden"` or `{"error":…}` when it cannot be opened.
- The wire has a fifth exchange, the notifications stream. The CLI does not carry it.

## Files in the container

All paths in bodies are arrays of components from the container's root (`["workspace","out.txt"]`), never
strings with `/`. No component may be empty, `.` or `..`.

```bash
# the whole tree, once (add --watch to keep getting changes)
diverge run agents filetree '{"agent":{"name":"echo"}}'

# download a file, or a directory (path [] is the whole root)
diverge run agents download --path ./out.txt '{"agent":{"name":"echo"},"path":["workspace","out.txt"]}'
diverge run agents download --path ./ws      '{"agent":{"name":"echo"},"path":["workspace"]}'

# upload one file; --path is the local file
diverge run agents upload --path ./in.txt '{"kind":"file","agent":{"name":"echo"},"path":["workspace","in.txt"]}'

# upload a directory; --path is the local directory, "files" names each file beneath it
diverge run agents upload --path ./src '{"kind":"directory","agent":{"name":"echo"},"path":["workspace","src"],"files":["main.py","lib/util.py"]}'

# copy container → agent | tool | dependency tool | volume, without the bytes reaching you
diverge run agents transfer '{"agent":{"name":"echo"},"path":["workspace"],"to":{"volume":{"provider":{"issuer":"…","subject":"github.12345.provider"},"name":"notes"},"path":["backup"]}}'
```

- A container that is not running is **started for the operation and stopped after it**, so these read the
  image plus the mounts, not the previous run's own disk.
- `filetree` shows the whole container including FUSE mounts; one snapshot line, then changes under `--watch`.
- `download` prints one line per file written. `"not_found"` is no such agent or nothing at the path. A
  directory with no files prints nothing.
- `upload` replaces each file whole and makes parents. A directory's `files` are `/`-separated paths from the
  directory's root; at least one, none twice. Answers `"uploaded"`, `"not_found"`.
- `transfer` destinations: `{"agent":<ref>,"path":[…]}`, `{"tool":<ref>,"path":[…]}`,
  `{"dependency":{"template":…,"owner":…},"path":[…]}`, `{"volume":{"provider":…,"name":…},"path":[…]}`.
  Answers `"transferred"`, `"not_found"`, `"no_destination"`, `"held"` (a volume at either end is in use by a
  running container or another file operation). Existing files at the destination that the source lacks stay.

## exec, terminal, frontend

```bash
# one command, no shell in between; JSON lines by default
diverge run agents exec '{"agent":{"name":"echo"},"argv":["ls","-la"],"cwd":["workspace"],"env":{"LANG":"C"}}'
# {"stdout":"<base64>"} … {"exited":0}

# the command's own output and exit code
diverge run agents exec --raw '{"agent":{"name":"echo"},"argv":["sh","-c","cat /etc/os-release | head -3"]}'

# an interactive shell: stdin goes to it, its output to stdout, exit code is the shell's
diverge run agents terminal '{"agent":{"name":"echo"},"cols":120,"rows":40}'
printf 'ls /\nexit\n' | diverge run agents terminal '{"agent":{"name":"echo"},"cols":120,"rows":40}'

# one HTTP request to the page the agent's program serves; body is stdin
diverge run agents frontend '{"agent":{"name":"me"},"method":"GET","path":"/frontend/","headers":[]}' </dev/null
```

- `exec`: `argv` is required and run as is (use `["sh","-c","…"]` for pipes). `cwd` and `env` are optional.
  Without `--raw`, stdout and stderr arrive base64-encoded, ending with `{"exited":<code>}` or
  `{"signaled":<signal>}`. Ending the command kills the one in the container.
- `terminal`: `cols` and `rows` are required. When stdin ends the shell receives Ctrl-D. Piped use is
  possible but output is a terminal's (escape codes). Prefer `exec` for scripted work.
- `frontend`: only paths beginning `/frontend` are reachable. `headers` is a list of `[name, value]` pairs.
  The response head is one JSON line on **stderr**, the body raw on **stdout**; exit `0` for a whole
  response. `"no_frontend"` means the image declares none (its `DIVERGE.md` has no `"frontend": true`).
- All three start a stopped container and hold it up while they run.
- `exec` and `terminal` both need the `exec` grant; `frontend` needs `frontend`.

## Grants

Each leaf needs a grant of the same name over agents: `create` (making), and over existing agents `get`,
`list`, `edit`, `delete`, `message`, `logs`, `download`, `upload`, `transfer`, `filetree`, `exec` (also
`terminal`), `frontend`, `connect`, plus `tag`/`untag`. Templates: `create`, `get`, `list`, `delete`,
`attach`, `tag`/`untag` over `agents_templates`. A create or edit that names a volume also needs `mount` over
the volume; one that names a vault key needs `use` over the key; a transfer needs `transfer` over the source
and `upload` over the destination. `"forbidden"` means no grant reaches what the request names, and nothing
changed. A list is never partially forbidden: it sends only what the account's `list` grants reach. See
`diverge-access`.

## Reference files

- [references/mounts.md](references/mounts.md) — provider pin, volume mounts, FUSE file and directory mounts,
  volume modes, edit patches for mounts.
- [references/answers.md](references/answers.md) — every leaf with its body and every answer it can print.

## Not verified

- The exact JSON of the line `download` prints per file written (it carries the file's path and byte count;
  run `agents download response-schema`).
- That a stopped container's own disk is discarded (so only mounts, the tools' databases and the log carry
  state between runs). This follows from the template and create documentation ("its state is its
  conversation … and nothing on any provider's disk") but was not tested.
- `agents frontend` and `agents terminal` were not run live while writing this; behaviour is from the CLI and
  SDK source.
- The installed binary's `agents message chat response-schema` may still describe a cancel; the source has none,
  and the rules above follow the source.

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
