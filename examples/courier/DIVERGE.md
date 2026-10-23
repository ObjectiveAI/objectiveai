---
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Arguments",
  "description": "The arguments a courier tool is made with.\n\nNone: the courier is the same for every caller, and what it keeps\nis told apart by the template the tool was made from. `{}` is the\none value; any member is refused, so a misspelled setting is never\nsilently ignored.\n\n# How to run it\n\nAs ONE `service`-mode standalone tool that every agent taking part\nis attached to. The mail lives in the caller's Postgres, and only\nagents attached to the same courier share it.",
  "type": "object",
  "additionalProperties": false
}
---
# Courier

An inbox between agents. Agents open conversations with one another
on a topic and reply inside them; the mail is kept in the caller's
database, and none of it is put into an agent's own conversation.
When a message arrives, the courier sends the recipient exactly this,
and nothing else:

```text
<courier>
You have a new message
</courier>
```

The agent reads its mail when it chooses, through four tools:

| tool | parameters | does |
|---|---|---|
| `start` | `to` (`{"name": …}` or `{"template": …, "index": n}`), `topic`, `text` | opens a conversation with another agent on a topic, with its first message |
| `reply` | `conversation`, `text` | adds a message to one of the caller's conversations |
| `conversations` | `include_read` (default false) | the caller's conversations with unread messages, newest first; all of them with `include_read` |
| `messages` | `conversation`, `include_read` (default false) | one conversation's unread messages, oldest first, then marks them read; the whole conversation with `include_read` |

A conversation is between two agents. Its topic is set when it is
opened and never changes. Every message records its sender as the
daemon attests the caller, never as the caller claims. A send whose
notification cannot be sent fails, and nothing is kept.

## Running it

- **Arguments:** none; `{}`.
- **Secrets:** none.
- **Template:** a standalone tool template from this image with
  `mode: service`. The courier is one shared service: the mail lives
  in the caller's Postgres, and only agents attached to the same
  courier tool share it. Couriers made from different templates keep
  separate mail.
- **Account:** create the tool under a named account whose roles let
  it get and message the agents that use it (`agents` `get` and
  `message`). The courier looks a recipient up by name or by template
  and index, and sends the notification, as that account.
- **Agents:** attach the courier tool to every agent that takes part.
  Only agents may use it; a client over a connect is refused.
