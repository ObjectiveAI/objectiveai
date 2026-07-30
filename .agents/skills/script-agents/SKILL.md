---
name: script-agents
description: Author an ObjectiveAI SCRIPT agent — Python that runs on the client's embedded runtime, receives the whole conversation as `input`, and returns assistant/tool messages. Use when you need a deterministic agent with no model in the loop, e.g. to drive a plugin tool from a test or to script a fixed sequence of tool calls.
---

# Authoring a script agent

A script agent replaces the model with Python. It runs on the client's
embedded runtime — the same one `objectiveai python` uses — so there is
no upstream call, no token cost, and no nondeterminism. That makes it
the right tool for **driving a specific tool call on demand**: testing a
plugin, reproducing a bug, or scripting a fixed sequence.

## The shape

```json
{
  "upstream": "script",
  "output_mode": "instruction",
  "type": "python",
  "python": "…source…",
  "plugins": [
    { "owner": "exampleorg", "name": "plugin-scaffold", "version": "v0.1.0" }
  ]
}
```

Four fields are load-bearing and all four are required in practice:

- **`"upstream": "script"`** — the marker that selects this agent kind.
- **`"output_mode": "instruction"`** — the only value the enum has.
  Omitting it does NOT default; the whole agent then fails to
  deserialize with a confusing untagged-enum error
  (`data did not match any variant of untagged enum
  InlineAgentBaseWithFallbacksOrRemoteCommitOptional`). If you see that
  error, this is the first thing to check.
- **`"type": "python"`** — the script language marker.
- **`"python"`** — the source, preserved verbatim. Whitespace is
  significant; it is never normalized.

`plugins`, `mcp_servers` and `laboratories` are optional and are what
give the script tools to call. A script with no plugins can still
speak, it just has nothing to call.

## The contract

**In:** a global named `input` — the FULL conversation so far, as a
list of message dicts (`{"role": …, "content": …, "tool_calls": …}`),
continuation included.

**Out:** the value of the **last expression**, which must be a list of
output messages. Only `assistant` and `tool` roles are allowed — a
script speaks as the assistant or records a tool result; it never puts
words in the user's mouth.

Both halves are worth verifying directly when in doubt:

```bash
objectiveai python --code 'input["a"] + 1' --input '{"a": 41}'   # -> 42
objectiveai python --code 'x = [1,2,3]
[i*2 for i in x]'                                                # -> [2,4,6]
```

Define a function and **call it on the last line** — the call is the
last expression, so its return value is the output:

```python
import json

def build():
    # Second pass: the tool has answered, so say something and stop.
    for m in reversed(input):
        if m.get("role") == "tool":
            return [{"role": "assistant", "content": "TOOL RESULT: " + str(m.get("content"))}]
    # First pass: call the tool.
    return [{
        "role": "assistant",
        "tool_calls": [{
            "id": "call_1",
            "type": "function",
            "function": {
                "name": "objectiveai-plugin-scaffold_scaffold_note_write_deleteme",
                "arguments": json.dumps({"key": "probe", "value": "hello"})
            }
        }]
    }]

build()
```

That is the whole idiom: **inspect `input` to decide which turn you are
on**, emit a tool call on the first pass, and a plain assistant message
once the tool result appears. Without that check the script emits the
same tool call forever.

## Rules that bite

- **`import` what you use.** The runtime is a real Python; `json` is
  NOT pre-imported. `json.dumps` without `import json` fails with
  `NameError: name 'json' is not defined`.
- **Tool names are prefixed.** A plugin's tool is exposed as
  `<plugin-name>_<tool-name>` — e.g. the scaffold's
  `scaffold_note_write_deleteme` is called as
  `objectiveai-plugin-scaffold_scaffold_note_write_deleteme`. The
  prefix derives from the plugin's NAME, not its owner. Get it wrong
  and the call silently finds no tool.
- **`arguments` is a JSON STRING**, not an object — hence
  `json.dumps({...})`.
- **Write the agent to a file** and pass `--agent-file`. Embedding a
  multi-line Python string in a shell argument is where quoting goes to
  die.

## Running it

```bash
objectiveai agents spawn --agent-file ./my-agent.json --simple "go"
```

See the `agent-control` skill for reading what it did — in particular
the `--target instance=…,parent=daemon` shape, which is the one thing
that reliably wastes time.
