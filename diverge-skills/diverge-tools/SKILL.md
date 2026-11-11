---
name: diverge-tools
description: Diverge tools (MCP-server containers served to agents) through `diverge run standalone-tools` and `diverge run dependency-tools` — standalone tool templates (`standalone-tools templates create|get|list|delete|tag|untag`, `mode` transient or service), standalone tools (`standalone-tools create|get|list|edit|delete|tag|untag`), attaching and detaching a tool to one agent or to an agent template (`attach-agent-instance`, `attach-agent-template`, `detach-agent-instance`, `detach-agent-template`), dependency tools an agent's image declares (transient vs service, scope instance/template/global, owner; `dependency-tools get|list|delete|tag|untag`), how an agent sees a tool's MCP tools (name prefixes), `standalone-tools connect list-tools|call-tool|list-resources|read-resource`, tool logs, a tool's permissions, and tool files/exec/terminal/frontend (`filetree|download|upload|transfer|exec|terminal|frontend` in both families). Load it when asked to give an agent a tool, make, attach, detach, inspect, debug or delete a tool, or work with the example tools courier, taskboard, memory, alarm, chat.
---

# Diverge tools

A **tool** is a container running an MCP server. The daemon runs it and serves its MCP tools and resources
to agents: an agent's program sees one merged tool list, and the daemon routes each call to the container
that serves it. A tool never calls an agent's tools; it may ask the daemon for things (for example to message
an agent) within the permissions its own program declares.

There are three kinds. They are separate families: a request of one family never finds a tool of another.

| kind | made by | lives | commands |
|---|---|---|---|
| **standalone tool** | you: a tool template, then `standalone-tools create`, then an attach | a record until deleted; one container however many agents it is attached to | `diverge run standalone-tools …` |
| **dependency tool** | the agent's image: its program declares a dependency template when its container registers, and the daemon deploys it | transient: one per agent run. service: kept under its scope | `diverge run dependency-tools …` (inspect only; nothing here makes one) |
| **connected standalone tool** | you: a record here pointing at a standalone tool on another daemon | a record here; the container is the other daemon's | `diverge run connected-standalone-tools …` — see `diverge-daemons` |

Every command is `diverge run <family> <leaf> '<JSON body>'`, one compact JSON line per answer. Run
`<leaf> request-schema` / `<leaf> response-schema` for exact shapes. Every body may carry `relay`
(`diverge-daemons`). Agents and agent templates are in `diverge-agents`; images and each image's
`DIVERGE.md` are in `diverge-images`.

## Naming a tool

- Standalone tool, `"tool": <reference>`: `{"name":"chat"}` (optional name; unique across standalone **and**
  connected tools together; a tool and an agent may share a name) or `{"template":"<tool template id>","index":2}`
  (its number among all tools ever made from that template; always works, never reused).
- Dependency tool, `"dependency": {"template":"<dependency template id>","owner":<owner>}` where owner is
  `{"agent":{"name":"alice"}}`, `{"agent":{"template":"<agent template id>","index":1}}`,
  `{"agent_template":{"template":"<agent template id>"}}`, or the string `"global"`.
- Agents are named as in `diverge-agents`: `{"name":…}` or `{"template":…,"index":…}`.

## Standalone tools: the whole path

```bash
# 1. a tool template (type "tool"); answers {"created":"<id>"} or {"exists":"<id>"}
TPL="$(diverge run standalone-tools templates create --jq '.created // .exists' '{
  "type": "tool",
  "mode": "service",
  "image": {"digest": "sha256:…", "references": [{"registry": "docker.io", "name": "diverge-local/chat"}]},
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {}
}' | tr -d '"')"

# 2. a tool from it; nothing runs yet
diverge run standalone-tools create "{\"template\":\"$TPL\",\"name\":\"chat\"}"          # "created"

# 3. attach it: to one agent, or to every agent of an agent template
diverge run standalone-tools attach-agent-instance '{"tool":{"name":"chat"},"agent":{"name":"echo"}}'   # "attached"
diverge run standalone-tools attach-agent-template '{"tool":{"name":"chat"},"template":"<agent template id>"}'

# 4. see it as the agent sees it
diverge run agents connect list-tools '{"agent":{"name":"echo"}}'
```

### Tool templates

The same shape as an agent template (`image`, `memory` and `disk` in bytes, `arguments`, optional `secrets`
names and `description`; id = SHA-256 of the canonical JSON; image references not in the hash; cannot be
edited) with two differences:

- `"type":"tool"`. Agent templates and tool templates are two namespaces; an id of one is not found by the
  other family.
- `"mode"`: how every tool made from it is kept running. Absent is `"transient"`.
  - `transient`: the container runs while it is used (an attached agent's loop or call, a connect, a file
    operation, exec, terminal, frontend) and is stopped `idle_seconds` (default 10) after the last use. An
    attachment alone keeps nothing running.
  - `service`: the container is kept running while the tool is attached to any agent or template, or a
    connect is open on it: started at the first attach, restarted if it ends, started at daemon boot. It is
    retired `idle_seconds` after the last attachment or connect leaves. Use it for tools that hold live
    state between calls (open sockets, timers).

`arguments` is whatever the image's `DIVERGE.md` schema says; the example tools all take `{}`.

| command | body | answers |
|---|---|---|
| `standalone-tools templates create` | the template | `{"created":"<id>"}`, `{"exists":"<id>"}` |
| `standalone-tools templates get` | `{"id":"<id>"}` | `{"found":{id, created, creator, tags, template}}`, `"not_found"` |
| `standalone-tools templates list` | `{}` or `ids`, `creators`, `in_use`, `all_tags`, `any_tags`, `created_from`, `created_to`, `count` | `{"added":…}` each, then `"listed"` (`--watch` keeps it open) |
| `standalone-tools templates delete` | `{"id":"<id>"}` | `"deleted"`, `"not_found"`, `"in_use"` (a tool was made from it) |
| `standalone-tools templates tag` / `untag` | `{"id":"<id>","tags":[…]}` | `"tagged"` / `"untagged"`, `"not_found"` |

### Create, get, list, edit, delete, tag

`standalone-tools create` takes exactly what `agents create` takes: `template` (required), optional `name`,
`secrets` (each template secret name → vault key), `provider` (pin + `volume_mounts`), `fuse_file_mounts`,
`fuse_directory_mounts`. Mount shapes and rules are the agents' own: see the `diverge-agents` skill's
`references/mounts.md`. Answers `"created"`, `"in_use"` (the name is a standalone or connected tool's
already), `"forbidden"`, `{"error":…}`. A create runs nothing.

```bash
diverge run standalone-tools get  '{"tool":{"name":"chat"}}'                 # {"found":{…}} | "not_found"
diverge run standalone-tools list '{"attached_agent_instances":["echo"]}'    # {"added":…}… "listed"
diverge run standalone-tools edit '{"tool":{"name":"chat"},"patch":[{"op":"replace","path":"/name","value":"lobby"}]}'
diverge run standalone-tools tag  '{"tool":{"name":"chat"},"tags":["ui"]}'   # "tagged"
diverge run standalone-tools untag '{"tool":{"name":"chat"},"tags":["ui"]}'  # "untagged"
diverge run standalone-tools delete '{"tool":{"name":"chat"}}'               # "deleted" | "not_found" | "attached"
```

- A list item carries `name`, `template`, `index`, `mode`, `provider`, `id` (container id while it runs),
  `creator`, `created`, `active`, `last_active`, `deployed`, `last_deployed`, `attached_agent_instances`,
  `attached_agent_templates`, `permissions`, `tags`. Filters: `names`, `templates`, `creators`, `modes`,
  `active`, `deployed`, `attached_agent_instances` (agent names; attached to every one),
  `attached_agent_templates`, `all_tags`, `any_tags`, `created_from`, `created_to`, `count`.
- `edit` is a JSON Patch over `name`, `volume_mounts`, `fuse_file_mounts`, `fuse_directory_mounts`,
  `secrets`. Answers `"edited"`, `"not_found"`, `"in_use"`, `"active"`. While the container runs, only FUSE
  mounts appended are accepted; anything else is `"active"`. A service tool that is attached is always
  running, so detach it (and wait `idle_seconds`) before any other edit.
- `delete` answers `"attached"` while the tool is attached to any agent or agent template: detach it
  everywhere first (`get` lists where). Delete is irreversible; the name is free afterwards.

### Attach and detach

| command | body | answers |
|---|---|---|
| `standalone-tools attach-agent-instance` | `{"tool":<ref>,"agent":<ref>}` | `"attached"`, `"no_tool"`, `"no_agent"` |
| `standalone-tools detach-agent-instance` | `{"tool":<ref>,"agent":<ref>}` | `"detached"`, `"no_tool"`, `"no_agent"` |
| `standalone-tools attach-agent-template` | `{"tool":<ref>,"template":"<agent template id>"}` | `"attached"`, `"no_tool"`, `"no_template"` |
| `standalone-tools detach-agent-template` | `{"tool":<ref>,"template":"<agent template id>"}` | `"detached"`, `"no_tool"`, `"no_template"` |

- Idempotent: attaching twice, or detaching what is not attached, is not a failure.
- Takes effect at once, on a running agent too: its tool list changes immediately and the agent is told so.
  A call already in flight is still answered.
- A template attach covers every agent of that template, those already made and those made later.
- An agent attached both directly and through its template has the tool once. Each attachment is removed
  the way it was made: a `detach-agent-instance` leaves a template attachment in place, and the reverse.
- One container serves every agent the tool is attached to. The tool tells callers apart by the caller the
  daemon attests on each request, never by what the caller claims.
- Grants: `attach` / `detach` over the tool, plus `edit` over the agent (instance) or `attach` over the agent
  template (template).

## Dependency tools

A dependency tool is declared by the **agent's program**, not by you and not by the agent template's own
members. For the agent-loop images in this repository the declaration is the agent template's
`arguments.dependencies`: a list of dependency templates, each deployed by the daemon when the agent's
container registers, before the agent's first turn.

```json
"arguments": {
  "model": "…",
  "dependencies": [
    {"image": {"digest": "sha256:…", "references": [{"registry": "docker.io", "name": "diverge-local/memory"}]},
     "memory": 268435456, "disk": 268435456, "arguments": {}, "mode": "transient"},
    {"image": {"digest": "sha256:…", "references": [{"registry": "docker.io", "name": "diverge-local/courier"}]},
     "memory": 268435456, "disk": 268435456, "arguments": {}, "mode": "service", "scope": "global"}
  ]
}
```

| mode | lives | owner | database |
|---|---|---|---|
| `transient` | deployed with each run of the agent, stopped with it | that agent | the agent's own, kept between runs |
| `service`, `scope: "instance"` | kept while its agent declares it | that agent | one per agent |
| `service`, `scope: "template"` | one for every agent of the same agent template that declares it | the agent template | shared by those agents |
| `service`, `scope: "global"` | one for every agent that declares it, from any agent template | nobody (`"global"`) | shared by all of them |

- The dependency template's **id** is the SHA-256 of its canonical JSON (mode and scope included). The same
  declaration under the same owner is the same tool, joined and not made again. Change any member and it is
  a different tool with a different database.
- A transient dependency may also mount the agent's own paths live: `fuse_file_mounts` /
  `fuse_directory_mounts`, each `{"agent_path":[…],"tool_path":[…]}`. A service mounts nothing of an agent's.
- A dependency's `secrets` are names; each must be a name the agent's template also declares, and is
  resolved through the agent's own binding.
- Every registration **resets** what the agent depends on to exactly what it declared then. A service no
  agent declares any more is retired after `idle_seconds`; its record and database stay until deleted.
- A dependency that will not deploy fails the agent's start: the message answers `{"error":…}` and the
  agent's log keeps an `error` naming the dependency.
- An instance tool is deleted with its agent, a per-template tool with its agent template. A global tool
  goes only with `dependency-tools delete`.
- A tool declares no dependencies of its own.

```bash
diverge run dependency-tools list                                   # {"added":…}… "listed"
diverge run dependency-tools list '{"modes":["service"],"in_use":true}'
diverge run dependency-tools get '{"dependency":{"template":"<id>","owner":"global"}}'
diverge run dependency-tools tag '{"dependency":{"template":"<id>","owner":{"agent":{"name":"echo"}}},"tags":["keep"]}'
diverge run dependency-tools untag '{"dependency":{"template":"<id>","owner":"global"},"tags":["keep"]}'
diverge run dependency-tools delete '{"id":"<dependency template id>"}'   # "deleted" | "not_found" | "in_use"
```

`delete` takes the template's `id` (not a reference) and removes **every** tool of that template under
every owner, each with its database and what it held. It answers `"in_use"` while any of them runs: wait for
the agents to go idle and retry. An agent that declares the template again at its next start gets a new,
empty tool. List item fields and filters are in
[references/dependency-tools.md](references/dependency-tools.md).

## How an agent sees a tool's MCP tools

The agent's program sees one MCP server. Every tool in it is exposed as `<prefix>_<tool name>`, and a call
is routed by the text before the first `_`:

| served tool | prefix |
|---|---|
| named standalone or connected tool | its name folded to `[a-z0-9-]`: lowercased, every other run of characters one `-` (`"My Notes"` → `my-notes`) |
| unnamed standalone tool | first 8 hex characters of its tool template id, `-`, its index (`3fa9c2d1-1`) |
| unnamed connected tool | `connected-<daemon record id>-<index>` |
| dependency tool | first 8 hex characters of its dependency template id (`b6dd4f61`) |

- Two tools folding to the same prefix: the later one gets `name-2`, `name-3`, …. Direct attachments come
  first in attach order, then the template's, so prefixes are stable from run to run.
- Example: tool `notebook` running the memory image exposes `notebook_remember`, `notebook_recall`,
  `notebook_forget`. The same image as a dependency exposes `b6dd4f61_remember` and so on.
- Renaming a standalone tool changes its prefix, and so the names an agent's prompt may rely on.
- Resources are not renamed: a read is by URI.
- Each listed tool, resource and result carries who serves it under `_meta` → `diverge.network`:
  `{"kind":"created"|"connected"|"dependency","image":…,"template":…}`. Each request reaching a tool carries
  the caller there the same way: `{"kind":"agent",…}` or `{"kind":"client",…}`. The daemon sets these; values
  sent from outside are replaced.

To see exactly what an agent has, list as the agent: `diverge run agents connect list-tools '{"agent":…}'`.

## `standalone-tools connect`: MCP straight into one tool

This is the scope another daemon uses to reach a tool here (a connected standalone tool there). From the
CLI it runs one MCP exchange against the tool's own server, unprefixed, as the client:

```bash
diverge run standalone-tools connect list-tools     '{"tool":{"name":"notebook"}}'
diverge run standalone-tools connect call-tool --name remember --arguments '{"text":"hi"}' '{"tool":{"name":"notebook"}}'
diverge run standalone-tools connect list-resources '{"tool":{"name":"notebook"}}'
diverge run standalone-tools connect read-resource --uri 'memo://1' '{"tool":{"name":"notebook"}}'
```

- Starts the tool's container if it is down and holds it while the exchange runs. Prints the MCP result's
  JSON; `--cursor` pages the two lists. Cannot be opened → `"not_found"`, `"forbidden"`, `{"error":…}`.
- Names here are the tool's own (`remember`), not the prefixed names an agent sees.
- The tool sees the caller as `kind: client` (your account's identity), not as an agent. A tool may refuse
  that: courier, taskboard and alarm are for agents only and refuse a client over a connect. To call a tool
  **as an agent**, use `diverge run agents connect call-tool` with the prefixed name.
- The wire's fifth exchange, the notifications stream, is not carried by the CLI. Notifications a tool sent
  are in its log.
- Needs the `connect` grant over the tool. There is no `dependency-tools connect`: reach a dependency tool
  through its agent.

## Tool logs

The daemon keeps a log per tool of every call and read it routed into it. Read it with the root `logs`
command (`diverge-logs` has every filter):

```bash
diverge run logs '{"of":{"type":"standalone_tool","name":"notebook"}}'
diverge run logs '{"of":{"type":"dependency_tool","template":"<id>","owner":"global"},"type":"call_error"}'
diverge run logs '{"of":{"type":"standalone_tool","name":"notebook"},"watch":true}'
```

Item types: the lifecycle `deployed`, `active`, `inactive`, `undeployed` (as for an agent; a tool is
`active` while a call or read is in flight); `call` then `call_result`, `call_error` or `call_interrupted`;
`read` then `read_result`, `read_error` or `read_interrupted`; `notification`; `error`; `command`. Each call
records the caller the daemon attested. A dependency tool's log is readable whether or not it runs. Needs the
`logs` grant over the tool.

## A tool's permissions

A tool's program declares, each time its container starts, what it may ask of the daemon: a list of grants
that name nothing. They reach by tags, or `"any"`, or over agents by two special words: `"attached"` (the
agents attached to this tool, or for a dependency the agents that declare it) and `"never_active"` (agents
that have never run). Example, the alarm tool: `[{"agents":{"actions":["message"],"within":"attached"}}]`.

- You do not set these; the image does. They are reported as `permissions` on the tool's list item after its
  first start (absent before, or when it declares none).
- The daemon holds the tool to exactly those. A tool that messages an agent it is not attached to is
  refused.
- There is no permission named `self` in the current source: the closest is `"attached"`.
- Every request a tool puts to the daemon is kept in the tool's own log as a `command` item: the request's
  path and body and `allowed` (`false` exactly when the daemon answered `forbidden`). Read these to see why a
  tool could not reach an agent.

## Files, exec, terminal, frontend

Both families have the same seven leaves as agents, with the same flags and answers (see `diverge-agents`
for the full description); only the reference member differs: `"tool":<ref>` for standalone,
`"dependency":{"template":…,"owner":…}` for dependency tools.

```bash
diverge run standalone-tools filetree '{"tool":{"name":"chat"}}'                       # --watch for changes
diverge run standalone-tools download --path ./db.bak '{"tool":{"name":"chat"},"path":["data","db.bak"]}'
diverge run standalone-tools upload --path ./seed.json '{"kind":"file","tool":{"name":"chat"},"path":["data","seed.json"]}'
diverge run standalone-tools transfer '{"tool":{"name":"chat"},"path":["data"],"to":{"agent":{"name":"echo"},"path":["import"]}}'
diverge run standalone-tools exec --raw '{"tool":{"name":"chat"},"argv":["ls","-la","/"]}'
diverge run standalone-tools terminal '{"tool":{"name":"chat"},"cols":120,"rows":40}'
diverge run standalone-tools frontend '{"tool":{"name":"chat"},"method":"GET","path":"/frontend/","headers":[]}' </dev/null

diverge run dependency-tools exec --raw '{"dependency":{"template":"<id>","owner":"global"},"argv":["env"]}'
diverge run dependency-tools download --path ./out '{"dependency":{"template":"<id>","owner":{"agent":{"name":"echo"}}},"path":["data"]}'
```

The one difference that matters:

| | container not running |
|---|---|
| standalone tool | **started** for the operation (file operations stop it again afterwards; exec, terminal and frontend hold it up while they run) |
| dependency tool | **never started**: every one of the seven answers `"not_found"` unless the tool is running. Message its agent first (or use a service that is up), then act within `idle_seconds`. As a transfer destination a stopped dependency tool is `"no_destination"`. |

- `frontend` reaches only paths under `/frontend`, and answers `"no_frontend"` when the image's `DIVERGE.md`
  does not declare one. Of the examples only `chat` (and the `human` agent) has a frontend. The response head
  is a JSON line on stderr and the body is raw on stdout.
- Grants: `download`, `upload`, `transfer`, `filetree`, `exec` (also `terminal`), `frontend`, each over the
  tool. For dependency tools the grant is over `dependency_tools`.
- A connected standalone tool's files and container are on the other daemon: none of these leaves find it.

## The example tools

Each is an image under `diverge-examples/` with a `DIVERGE.md` (read it with `diverge run images info`). All take
`arguments: {}` and no secrets. Details and the exact MCP tools of each are in
[references/example-tools.md](references/example-tools.md).

| example | what it is | meant to run as |
|---|---|---|
| `courier` | an inbox between agents; notifies the recipient by messaging it | dependency, `service`, `global` |
| `taskboard` | a shared board of tasks agents post, claim and complete | dependency, `service`, `template` or `global` |
| `memory` | an agent's notebook: remember, recall, forget | dependency, `transient` |
| `alarm` | an agent sets alarms that message it later (once or cron) | dependency, `service`, any scope |
| `chat` | a person chats with attached agents through the tool's frontend | standalone tool, `service`, has a frontend |
| `human` | not a tool: an **agent** image with no model, driven by a person at its frontend | agent (`diverge-agents`) |

## Pitfalls

- Creating a standalone tool does nothing for any agent until it is attached.
- A standalone-tools request never finds a dependency or connected tool, and the reverse. Pick the family by
  how the tool came to be.
- Deleting order: detach → delete tool → delete tool template. Each earlier step blocks the later one
  (`"attached"`, `"in_use"`).
- `standalone-tools connect call-tool` uses the tool's own names; `agents connect call-tool` uses prefixed
  names. Mixing them gives an MCP error, not a Diverge one.
- The first message to an agent with dependencies can fail while they are still deploying ("connection ended
  before the channel answered"). Send it again.
- `"forbidden"` on an attach may be about the agent (`edit`) or the template (`attach`), not the tool. See
  `diverge-access`.

## Reference files

- [references/dependency-tools.md](references/dependency-tools.md) — the dependency template shape, owners,
  list item fields and filters, lifetimes.
- [references/answers.md](references/answers.md) — every leaf of both families with its body and answers.
- [references/example-tools.md](references/example-tools.md) — the example tools' MCP tools, permissions and
  worked setups.

## Not verified

- Running `memory`, `courier`, `taskboard` or `alarm` as a **standalone** tool: their `DIVERGE.md` documents
  them as dependency tools. Both kinds are MCP tool containers, so it is expected to work for `memory`, but
  the three agent-only tools decide for themselves which callers they accept.
- `standalone-tools terminal`, `frontend` and every `dependency-tools` container leaf were not run live while
  writing this; behaviour is from the CLI and SDK source.
- Whether a dependency tool's exposed prefix escalates (`-2`) when two dependency template ids share their
  first 8 characters; the prefix registry is shared, so it should.
- The task brief mentions a `self` permission; no such word exists in the SDK's permission types at this
  commit.

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
