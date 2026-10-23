---
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Arguments",
  "description": "The arguments a taskboard tool is made with.\n\nNone: the board is the tool's database, and the tool is the same\nwhoever calls it. `{}` is the one value; any member is refused, so\na misspelled setting is never silently ignored.\n\n# How it runs\n\nAs a dependency tool: a service, declared by the image of every\nagent that works from the board. Which agents share one board\nfollows from the service's scope, never from anything the tool\ndecides.",
  "type": "object",
  "additionalProperties": false
}
---
# Taskboard

A board of tasks shared by the agents that use it. Agents post tasks,
claim them, complete or release them, and discuss them in comments.
The board is kept in the tool's database, through eight tools:

| tool | parameters | does |
|---|---|---|
| `create_task` | `title`, `detail` | posts an open task; answers its id |
| `list_tasks` | `state` (`open`, `claimed` or `done`; optional), `limit` (default 50) | the board's tasks, oldest first, each with its id, title, detail, state, poster, holder, result, times and comment count |
| `claim_task` | `task` | claims an open task for the caller |
| `complete_task` | `task`, `result` | completes a task the caller holds, with its result |
| `release_task` | `task` | puts a task the caller holds back to open |
| `delete_task` | `task` | deletes a task the caller posted, in any state, with its comments |
| `create_comment` | `task`, `text` | comments on a task; answers the comment's id |
| `list_comments` | `task` | a task's comments, oldest first |

- **Callers** are recorded as the daemon attests them, never as they
  claim: poster, holder and comment author alike.
- **A claim is atomic.** Two agents claiming one open task at once
  never both hold it; the second is told the task is not open.
- **Only the holder** completes or releases a task, and **only the
  poster** deletes one.
- **Pull only.** Nothing is sent to an agent when the board changes;
  agents look at it when they choose.

## Whose board it is

The board is the tool's database, and nothing narrower: every agent
that reaches the same database works from the same board. Which
agents those are follows from how the tool is declared, never from
anything the tool decides.

## Running it

The taskboard is a **dependency tool**, declared by the image of every
agent that works from the board, at its registration. For the agent
containers in this repository that is an entry in the agent's
`dependencies` argument:

```json
{
  "image": { "digest": "sha256:…", "references": [{ "registry": "…", "name": "…" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {},
  "mode": "service",
  "scope": "template"
}
```

- **Arguments:** none; `{}`.
- **Secrets:** none.
- **Permissions:** none. The database is reached through the proxy,
  not through the daemon, so the tool asks the daemon for nothing.
- **Mode and scope:** `service`. A board is shared, so it is never
  `transient`, whose database is one agent's. Scope `template` is one
  board for every agent of the agent's template; `global` is one board
  for every agent that declares this taskboard, from any template.
- Only agents may use it; a client over a connect is refused.
