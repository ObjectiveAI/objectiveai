# The example tools

Six images under `examples/` in the repository. Five are tools; `human` is an agent image listed here because
it is used with them. Each image's `/DIVERGE.md` states its name, its arguments schema and whether it serves
a frontend; read it from a running daemon with `diverge run images info` (`diverge-images`), which is also
where you get the digest to put in a template. All take `arguments: {}` (any member is refused) and no
secrets.

In every example below `<digest>` is the image's digest in the provider's store, and the `references` entry
must name a registry the provider lists (for locally built images that is typically `docker.io` with the
local repository name; nothing is pulled when the provider already holds the digest).

## memory — a notebook (dependency, transient)

| MCP tool | parameters | does |
|---|---|---|
| `remember` | `text`, `tags` (optional) | keeps a note; answers its id |
| `recall` | `query` (optional), `tags` (optional), `limit` (default 20) | notes carrying every tag given; with `query`, a text search, best match first; otherwise newest first |
| `forget` | `id` | deletes a note |

- Permissions: none. The notes are in the tool's database, which it reaches through its container's proxy,
  not through the daemon.
- The tool records no owner: whoever reaches the same database shares the notes. As a `transient`
  dependency the database is the declaring agent's own, and it persists between the agent's runs.

Declare it in an agent template (python agent-loop image shown; `openrouter`, `cc` and the others take the
same `dependencies` member):

```bash
diverge run agents templates create '{
  "type": "agent",
  "image": {"digest": "<python digest>", "references": [{"registry": "docker.io", "name": "diverge-local/python"}]},
  "memory": 536870912, "disk": 1073741824,
  "arguments": {
    "python": "…agent script…",
    "dependencies": [
      {"image": {"digest": "<memory digest>", "references": [{"registry": "docker.io", "name": "diverge-local/memory"}]},
       "memory": 268435456, "disk": 268435456, "arguments": {}, "mode": "transient"}
    ]
  }
}'
# → {"created":"<agent template id>"}; then agents create, agents message
diverge run agents connect list-tools '{"agent":{"name":"echo"}}'      # <8 hex>_remember, <8 hex>_recall, <8 hex>_forget
diverge run dependency-tools list '{"owners":[{"agent":{"name":"echo"}}]}'
```

## taskboard — a shared board (dependency, service)

| MCP tool | parameters | does |
|---|---|---|
| `create_task` | `title`, `detail` | posts an open task; answers its id |
| `list_tasks` | `state` (`open`, `claimed`, `done`; optional), `limit` (default 50) | the board's tasks, oldest first |
| `claim_task` | `task` | claims an open task for the caller (atomic: two claimers never both win) |
| `complete_task` | `task`, `result` | completes a task the caller holds |
| `release_task` | `task` | puts a task the caller holds back to open |
| `delete_task` | `task` | deletes a task the caller posted, with its comments |
| `create_comment` | `task`, `text` | comments on a task |
| `list_comments` | `task` | a task's comments, oldest first |

- Permissions: none. Pull only: nothing is sent to an agent when the board changes.
- Callers are recorded as the daemon attests them. Only agents may use it; a client over
  `standalone-tools connect` is refused.
- Which agents share a board is decided only by the scope: `"mode":"service","scope":"template"` is one
  board per agent template; `"scope":"global"` is one board for every agent that declares this exact
  template. Never `transient` (that database is one agent's).

## courier — an inbox between agents (dependency, service, global)

| MCP tool | parameters | does |
|---|---|---|
| `list_agents` | none | agents the caller can write to: those using this courier, then those that have never run |
| `create_conversation` | `to` (`{"name":…}` or `{"template":…,"index":n}`), `topic`, `text` | opens a conversation with another agent, with its first message |
| `reply` | `conversation`, `text` | adds a message to one of the caller's conversations |
| `list_conversations` | `include_read` (default false) | the caller's conversations with unread messages, newest first |
| `list_messages` | `conversation`, `include_read` (default false) | a conversation's unread messages, oldest first, and marks them read |

- On each send the courier messages the recipient agent exactly
  `<courier>\nYou have a new message\n</courier>`; the daemon starts the recipient if it is not running. The
  mail itself stays in the courier's database, read through the tools.
- Permissions it declares for itself:

  ```json
  [
    {"agents": {"actions": ["get", "list", "message"], "within": "attached"}},
    {"agents": {"actions": ["get", "list", "message"], "within": "never_active"}}
  ]
  ```

  `attached` is every agent whose last registration declared this courier; `never_active` is every agent
  that has never run, so one can be written to before its first start. An agent that has run and does not
  declare the courier is unreachable, and an agent whose image never declares it can be written to but can
  never read its mail.
- Declare it as `"mode":"service","scope":"global"` in **every** participating agent's template, with the
  byte-identical template: the dependency template's id is what the mail is kept under, so any difference
  (arguments, limits, digest) is a separate mail system.
- Only agents may use it; a client over a connect is refused.

## alarm — wake me later (dependency, service)

| MCP tool | parameters | does |
|---|---|---|
| `set_alarm` | `note`, and exactly one of `at` (RFC 3339), `in_seconds`, `cron` (five fields, UTC) | sets an alarm for the caller; answers its id and when it is first due |
| `list_alarms` | none | the caller's alarms, soonest first |
| `cancel_alarm` | `alarm` | cancels one of the caller's alarms |

- When due, the tool messages the agent that set it exactly `<alarm>\n{note}\n</alarm>`; the daemon starts
  the agent if it is idle.
- Permissions it declares: `[{"agents":{"actions":["message"],"within":"attached"}}]`.
- Must be `service` (a transient tool stops with the agent's run and would keep no time). Any scope works,
  since each alarm wakes only the agent that set it.
- An alarm that cannot be sent when due is dropped. Alarms that fell due while the tool was down fire once
  when it returns; missed repeats of a cron alarm are skipped.
- Only agents may use it; a client over a connect is refused.

## chat — a person talks to agents (standalone tool, service, frontend)

| MCP tool | parameters | does |
|---|---|---|
| `read_messages` | `include_read` (default false) | the person's messages to the calling agent, oldest first; unread ones are marked read |
| `send_message` | `text` | sends the person a message, shown in every open page |

- The person writes in the tool's frontend (one tab per attached agent). On each message the tool messages
  the agent exactly `<chat>\nYou have a new message\n</chat>`; the agent then calls `read_messages`.
- Its `DIVERGE.md` declares `"frontend": true`.
- Permissions it declares: `get`, `list` and `message` over the agents attached to it, `get` over agent
  templates, `info` over images.

```bash
TPL="$(diverge run standalone-tools templates create --jq '.created // .exists' '{
  "type": "tool", "mode": "service",
  "image": {"digest": "<chat digest>", "references": [{"registry": "docker.io", "name": "diverge-local/chat"}]},
  "memory": 268435456, "disk": 268435456, "arguments": {}
}' | tr -d '"')"
diverge run standalone-tools create "{\"template\":\"$TPL\",\"name\":\"chat\"}"
diverge run standalone-tools attach-agent-instance '{"tool":{"name":"chat"},"agent":{"name":"echo"}}'
diverge run standalone-tools get '{"tool":{"name":"chat"}}'            # deployed: true once the service is up
diverge run standalone-tools frontend '{"tool":{"name":"chat"},"method":"GET","path":"/frontend/","headers":[]}' </dev/null
diverge run agents connect list-tools '{"agent":{"name":"echo"}}'      # chat_read_messages, chat_send_message
```

The page is meant for a browser; the viewer (diverge-world) opens a tool's frontend tab. The CLI's `frontend`
leaf is one HTTP request at a time and is enough to check that the page is served.

Taking it down again:

```bash
diverge run standalone-tools detach-agent-instance '{"tool":{"name":"chat"},"agent":{"name":"echo"}}'
diverge run standalone-tools delete '{"tool":{"name":"chat"}}'
diverge run standalone-tools templates delete "{\"id\":\"$TPL\"}"
```

## human — an agent with no model (agent image, frontend)

Not a tool. An agent image whose "model" is a person: a message waits on its frontend until someone answers
there, writing the agent's replies, calling its attached tools by hand, and ending the turn. Nothing times
out. Make it like any agent (`agents templates create` with `arguments: {}`, `agents create`), attach the
tools the person should be able to call, and open its frontend (`agents frontend`, or the viewer). It is the
quickest way to exercise a tool exactly as an agent would without a model: so is
`diverge run agents connect call-tool` against any agent the tool is attached to.

## Which shape for which need

| need | use |
|---|---|
| private state for one agent that survives its restarts | a `transient` dependency (memory) |
| state shared by all agents of one template | a `service` dependency, `scope: "template"` (taskboard) |
| state or messaging shared across templates | a `service` dependency, `scope: "global"` (courier) |
| something that must keep running between agent runs | a `service`, dependency or standalone (alarm, chat) |
| a tool you pick per agent at runtime, or that has a page for people | a standalone tool, attached (chat) |
| a tool that lives on another daemon | a connected standalone tool (`diverge-daemons`) |
