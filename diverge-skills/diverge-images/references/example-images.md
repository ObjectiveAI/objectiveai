# The example images under `diverge-examples/`

Six small images that show what a tool or an agent container can be. All take `arguments` `{}`
(any member is refused, so a misspelled setting is never silently ignored) and declare **no secrets**.
Build each from the workspace root: `-f diverge-examples/tools/<name>/Containerfile .` (`diverge-examples/agents/human/Containerfile` for `human`) (see the main file).

Four are **dependency tools**: an agent declares them in its own `arguments.dependencies`, and the
daemon deploys them. One is a **standalone tool** made by hand. One is an **agent**.

| image | kind | mode / scope | permissions it declares at registration | database |
|---|---|---|---|---|
| `courier` | dependency tool | service, `global` | `get`, `list`, `message` over agents `attached` and over agents `never_active` | yes |
| `taskboard` | dependency tool | service, `template` or `global` | none | yes |
| `memory` | dependency tool | transient | none | yes |
| `alarm` | dependency tool | service, any scope | `message` over agents `attached` | yes |
| `chat` | standalone tool, frontend | service | `get`, `list`, `message` over agents `attached`; `get` over agent templates (any); `info` over images | yes |
| `human` | agent, frontend | — | — (an agent asks the daemon nothing) | yes |

A tool's permissions are declared by its own program each time it starts; neither a template nor an
agent states them. "Database" means the tool keeps its state in its own Postgres scope, reached
through the proxy (see `postgres` in `diverge-storage`); whose data it is follows from the scope.

In every dependency entry below, `<digest>` is that image's digest and the `references` entry names a
registry the provider lists.

## courier — an inbox between agents

Agents open conversations with one another on a topic and reply inside them. Mail is kept in the
courier's database, never put into an agent's conversation. When a message arrives the courier sends
the recipient exactly:

```text
<courier>
You have a new message
</courier>
```

and the daemon starts the recipient if it is not running.

| tool | parameters | does |
|---|---|---|
| `list_agents` | none | the agents the caller can write to: those that use this courier now, then those that have never run |
| `create_conversation` | `to` (`{"name":…}` or `{"template":…,"index":n}`), `topic`, `text` | opens a conversation with its first message |
| `reply` | `conversation`, `text` | adds a message to one of the caller's conversations |
| `list_conversations` | `include_read` (default false) | the caller's conversations with unread messages, newest first |
| `list_messages` | `conversation`, `include_read` (default false) | one conversation's unread messages, oldest first, then marks them read |

Dependency entry (put it in each participating agent's `arguments.dependencies`):

```json
{
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/courier" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {},
  "mode": "service",
  "scope": "global"
}
```

- One courier container and one database for every agent that declares this exact entry, from any
  agent template. The entry's hash is what the mail is kept under, so a different entry (another
  digest, other limits) is a different mail system.
- `attached` reaches agents whose last registration declared the courier; `never_active` reaches
  agents that have never run, so they can be written to before their first start. An agent that has
  run and does not declare the courier is reachable by neither.
- Every sender is recorded as the daemon attests the caller. Only agents may use it; a client over a
  `connect` is refused.

## taskboard — a shared board of tasks

Agents post tasks, claim them, complete or release them, and comment on them. Pull only: nothing is
sent to an agent when the board changes.

| tool | parameters | does |
|---|---|---|
| `create_task` | `title`, `detail` | posts an open task; answers its id |
| `list_tasks` | `state` (`open`, `claimed`, `done`; optional), `limit` (default 50) | the board's tasks, oldest first |
| `claim_task` | `task` | claims an open task for the caller (atomic: two claimants never both hold it) |
| `complete_task` | `task`, `result` | completes a task the caller holds |
| `release_task` | `task` | puts a task the caller holds back to open |
| `delete_task` | `task` | deletes a task the caller posted, with its comments |
| `create_comment` | `task`, `text` | comments on a task |
| `list_comments` | `task` | a task's comments, oldest first |

Dependency entry:

```json
{
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/taskboard" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {},
  "mode": "service",
  "scope": "template"
}
```

- `"scope":"template"` is one board for every agent made from the same agent template; `"global"` is
  one board for every agent that declares this taskboard. Never `transient`, whose database is one
  agent's.
- Only the holder completes or releases a task; only the poster deletes one. Only agents may use it.

## memory — a notebook that outlasts the conversation

| tool | parameters | does |
|---|---|---|
| `remember` | `text`, `tags` (default none) | keeps a note; answers its id |
| `recall` | `query` (optional), `tags` (default none), `limit` (default 20) | notes carrying every tag given; with `query`, those whose text matches it as a web search (quoted phrases, `or`, `-` to exclude), best match first; without, newest first |
| `forget` | `id` | deletes a note |

Dependency entry:

```json
{
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/memory" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {},
  "mode": "transient"
}
```

- Transient: deployed with each run of the agent and stopped with it. The notes stay in the database
  between runs, because a transient tool's database scope is its parent agent's.
- The tool tells no caller apart: every agent that reaches the same database shares the same notes.
  Whose notebook it is follows from the mode and scope, never from anything the tool decides.
- The setup report also runs this image as a standalone tool
  (`standalone-tools templates create` with `"type":"tool","mode":"transient"`); that use is assumed
  to work and was not run live.

## alarm — alarms an agent sets to wake itself

When an alarm is due the tool messages the agent that set it with exactly:

```text
<alarm>
{note}
</alarm>
```

The daemon starts the agent if it is not running, so an alarm wakes an idle agent.

| tool | parameters | does |
|---|---|---|
| `set_alarm` | `note`, and exactly one of `at` (RFC 3339 time), `in_seconds`, `cron` (five fields, UTC) | sets an alarm for the caller; answers its id and when it is first due |
| `list_alarms` | none | the caller's alarms, soonest first |
| `cancel_alarm` | `alarm` | cancels one of the caller's alarms |

Dependency entry:

```json
{
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/alarm" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {},
  "mode": "service",
  "scope": "global"
}
```

- A service, never transient: it keeps time between the agent's runs. Any scope works, since every
  alarm wakes only the agent that set it.
- Cron is five fields in UTC; a seconds or year field is refused. An alarm that cannot be sent when
  due is dropped (a recurring one moves on). Alarms that fell due while the tool was down fire once
  when it comes back; missed repeats are skipped. Only agents may use it.

## chat — a person talks to the attached agents (standalone tool, frontend)

The person writes in the tool's frontend, one tab per attached agent, and the agent is told exactly:

```text
<chat>
You have a new message
</chat>
```

| tool | parameters | does |
|---|---|---|
| `read_messages` | `include_read` (optional, false) | the person's messages to the caller, oldest first; unread ones handed back are marked read |
| `send_message` | `text` | sends the person a message, shown in every open page at once |

A standalone tool, made by hand. Tool template body for `diverge run standalone-tools templates create`:

```json
{
  "type": "tool",
  "mode": "service",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/chat" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {}
}
```

Then make a tool of the template (`standalone-tools create`), attach it to the agents that chat
(`standalone-tools attach-agent-instance` or `attach-agent-template`), and open its frontend (see
`diverge-tools`). A person's message carries the account the daemon stamped the page's request with;
an agent's, the agent the daemon attested. Every message is kept in the tool's database.

## human — an agent with no model: you are the agent (agent, frontend)

When someone messages it, the message waits on its frontend until a person answers. The person
writes the agent's replies, calls its tools by hand with JSON arguments, and ends the turn, which
sends everything done back as the agent's turn. Nothing times out. The transcript is kept in the
agent's database, so it survives the container starting again.

Agent template body for `diverge run agents templates create`:

```json
{
  "type": "agent",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/human" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {}
}
```

Then `agents create`, attach the tools you want to call as it, and open its frontend (the agent's
`frontend` commands in `diverge-agents`, or its frontend tab in diverge-world). Secrets: none.
