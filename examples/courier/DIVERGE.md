---
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Arguments",
  "description": "The arguments a courier tool is made with.\n\nNone: the courier is the same for every caller, and what it keeps\nis told apart by the template the tool was made from. `{}` is the\none value; any member is refused, so a misspelled setting is never\nsilently ignored.\n\n# How it runs\n\nAs a dependency tool: a service whose scope is `global`, declared\nby the image of every agent that takes part. One container, one\ndatabase, shared by every agent that declares it.",
  "type": "object",
  "additionalProperties": false
}
---
# Courier

An inbox between agents. Agents open conversations with one another
on a topic and reply inside them; the mail is kept in the courier's
database, and none of it is put into an agent's own conversation.
When a message arrives, the courier sends the recipient exactly this,
and nothing else:

```text
<courier>
You have a new message
</courier>
```

The agent reads its mail when it chooses, through five tools:

| tool | parameters | does |
|---|---|---|
| `list_agents` | none | the agents the caller can write to and be read by: those that use this courier now, then those that have never run, each with its name, template and index |
| `create_conversation` | `to` (`{"name": …}` or `{"template": …, "index": n}`), `topic`, `text` | opens a conversation with another agent on a topic, with its first message |
| `reply` | `conversation`, `text` | adds a message to one of the caller's conversations |
| `list_conversations` | `include_read` (default false) | the caller's conversations with unread messages, newest first; all of them with `include_read` |
| `list_messages` | `conversation`, `include_read` (default false) | one conversation's unread messages, oldest first, then marks them read; the whole conversation with `include_read` |

A conversation is between two agents. Its topic is set when it is
opened and never changes. Every message records its sender as the
daemon attests the caller, never as the caller claims. A send whose
notification cannot be sent fails, and nothing is kept.

## Running it

The courier is a **dependency tool**: a global service that every
agent taking part declares, through its own image, at its
registration. For the agent containers in this repository that is
an entry in the agent's `dependencies` argument:

```json
{
  "image": { "digest": "sha256:…", "references": [{ "registry": "…", "name": "…" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {},
  "mode": "service",
  "scope": "global",
  "permissions": [
    { "agents": { "actions": ["get", "list", "message"], "within": "self" } },
    { "agents": { "actions": ["get", "list", "message"], "within": "never_active" } }
  ]
}
```

- **Arguments:** none; `{}`.
- **Secrets:** none.
- **Mode and scope:** `service`, `global`. One courier container and
  one database for every agent that declares this template, from any
  agent template. The template's id is what the courier keeps its mail
  under, so a different template is a different mail system.
- **Permissions:** `self` reaches the agents attached to the courier,
  those whose last registration declared it; `never_active` reaches
  the agents that have never run, so they can be listed and written to
  before their first start, and the notification starts them. An
  agent that has run and does not declare the courier is reachable by
  neither, and cannot read mail. An agent that has never run, but
  whose image will not declare the courier, can be written to and
  never read it.
- Only agents may use it; a client over a connect is refused.
