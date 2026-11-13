---
{
  "name": "human",
  "chat": {
    "prompt_modalities": {
      "image": false,
      "audio": false,
      "file": false
    }
  },
  "structured_decisions": false,
  "schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "Arguments",
    "description": "The arguments a human agent is made with.\n\nNone: the agent is whoever answers on its frontend, and its tools\nare whichever are attached to it. `{}` is the one value; any member\nis refused, so a misspelled setting is never silently ignored.",
    "type": "object",
    "additionalProperties": false
  },
  "frontend": true
}
---
# Human

An agent with no model: **you are the agent.** When someone messages it,
the message waits on its frontend until you answer. You write the
agent's replies, call its tools by hand, and end the turn, which sends
everything you did back as the agent's turn.

- **The frontend** shows the whole transcript, live: every message that
  came in, every reply, every tool call with its result. A message that
  arrives while you are at it is shown at once and is part of the same
  turn.
- **Replies** are the agent's text; **Call** runs one of the agent's
  tools with the JSON arguments you give, prefilled from the tool's
  schema; **End turn** ends the run.
- **Nothing times out.** The agent waits on you for as long as it takes.
- **The transcript** is kept in the agent's database, so it survives the
  container starting again.

Secrets: none. Arguments: `{}`.

## Running it

Make an agent template of the image (`agents templates create`,
`arguments` `{}`), an agent of the template (`agents create`), attach
the tools you want to call as it, and open its **frontend** tab in
diverge-world.
