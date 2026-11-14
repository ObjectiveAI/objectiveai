---
{
  "name": "chat",
  "schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "Arguments",
    "description": "The arguments a chat tool is made with.\n\nNone: the chat is between a person and whichever agents are\nattached to the tool, and the tool is the same whoever calls it.\n`{}` is the one value; any member is refused, so a misspelled\nsetting is never silently ignored.\n\n# How it runs\n\nAs a standalone tool, a service: it keeps the sockets of every open\npage between the agents' calls.",
    "type": "object",
    "additionalProperties": false
  },
  "frontend": true
}
---
# Chat

A chat between a person and the agents attached to the tool. The
person writes in the tool's frontend — one tab per attached agent, each
on a live connection — and the agent is told a message is waiting,
exactly this and nothing else:

```text
<chat>
You have a new message
</chat>
```

The agent reads and answers with two tools:

| tool | parameters | does |
|---|---|---|
| `read_messages` | `include_read` (optional, `false`) | the person's messages to the caller, oldest first: the unread ones, or every one with `include_read`; every unread one handed back is marked read |
| `send_message` | `text` | sends the person a message, shown in every open page at once; answers its id |

- **Every message is kept** in the tool's database, so a page shows the
  whole history of each conversation.
- **Who is who.** A person's message carries the account the daemon
  stamped the page's request with; an agent's, the agent the daemon
  attested on the call. Nothing a page or an agent says of itself is
  trusted.
- **The frontend** shows each agent's name, its template and index, and
  its image's name and `DIVERGE.md`.

## Running it

A **standalone tool**, a service: make a template of the image
(`standalone-tools templates create`, `"mode":"service"`, `arguments`
`{}`), a tool of the template (`standalone-tools create`), and attach it
to the agents that chat (`standalone-tools attach-agent-instance` or
`attach-agent-template`). The tool asks the daemon for `get`, `list` and
`message` over the agents attached to it, `get` over agent templates,
and `read` over providers and remote daemons, through which it reads
each image's `/DIVERGE.md`.
