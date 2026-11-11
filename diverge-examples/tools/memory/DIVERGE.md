---
{
  "name": "memory",
  "schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "Arguments",
    "description": "The arguments a memory tool is made with.\n\nNone: the notebook is the tool's database, and the tool is the\nsame whoever calls it. `{}` is the one value; any member is\nrefused, so a misspelled setting is never silently ignored.\n\n# How it runs\n\nAs a dependency tool, declared by the image of the agent that\nkeeps the notes; meant to be transient. Whose notebook it is\nfollows from whose database the tool is given, never from anything\nthe tool decides.",
    "type": "object",
    "additionalProperties": false
  }
}
---
# Memory

A notebook that outlasts an agent's conversation. An agent keeps
notes, finds them again by a search of their text and by their tags,
and deletes them. The notes are kept in the tool's database, through
three tools:

| tool | parameters | does |
|---|---|---|
| `remember` | `text`, `tags` (default none) | keeps a note; answers its id |
| `recall` | `query` (optional), `tags` (default none), `limit` (default 20) | the notes carrying every tag given; with `query`, only those whose text matches it as a web search (quoted phrases, `or`, `-` to exclude), best match first; without, the newest first. Each with its id, text, tags and when it was kept |
| `forget` | `id` | deletes a note |

## Whose notes they are

The notebook is the tool's database, and nothing narrower: the tool
tells no caller apart and records no owner. Every agent that reaches
the same database shares the same notes; whose notebook it is follows
from how the tool is declared, never from anything the tool decides.

## Running it

The memory tool is a **dependency tool**, declared by the image of
the agent that keeps the notes, at its registration. For the agent
containers in this repository that is an entry in the agent's
`dependencies` argument:

```json
{
  "image": { "digest": "sha256:…", "references": [{ "registry": "…", "name": "…" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {},
  "mode": "transient"
}
```

- **Arguments:** none; `{}`.
- **Secrets:** none.
- **Permissions:** none. The database is reached through the proxy,
  not through the daemon, so the tool asks the daemon for nothing.
- **Mode:** `transient`. The tool is deployed with each run of the
  agent and stopped with it; the notes stay in the database between
  runs.
