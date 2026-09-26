# The python harness — settled decisions

The operating rules for this container's implementation, accumulated
as they are settled. Each entry is a decision already made. Add to
this file as more is figured out. The execution contract below is
the CLI's — `objectiveai-daemon/src/python.rs`, the RustPython-in-
wasm harness — carried to a real interpreter; nothing here has been
run in the image, and the harness has been run only on a host
Python.

## What is built

The whole container, never run as an image: the agent vocabulary
with its schema (`agent/`); the server (`main.rs`, openrouter's);
openrouter's claim, queue, continuation and history verbatim;
registration as the install (`registration.rs`); the harness
(`harness.py`, carried in the binary and written to disk at
registration); one run of the script (`run/`); and the loop
(`loop/`): list, run, check, call, seam.

## The agent

`{python, requirements}`. `python` is the source, verbatim, never
trimmed. `requirements` is package → `{operator, version}`, ordered,
one constraint per package, PEP 440's seven operators (`===` is
out). Dropped from the SDK's reference type: `upstream` (the image
is the discriminator), `memory` and `disk` (the container request's
ceilings, not the agent's vocabulary).

## Registration is the install

`POST /register` writes `/var/lib/diverge-python/harness.py` and
`agent.py`, then runs `python3 -m pip install --no-cache-dir
--break-system-packages name<op>version…` if there are any
requirements, and only then holds the agent. The answer waits for
pip, however long. A failed install is `400 {kind: "requirements",
error: {status, output}}` with everything pip said, and the agent
stays unregistered for a corrected one; the disk or pip failing to
start is `500`; a second registration is `409`. The whole handler
runs under one lock, so two registrations cannot interleave their
installs. Nothing of the proxy's is touched. System python, no venv,
by ruling: `--break-system-packages` holds on Debian's guarded
interpreter and on the official image's unguarded one alike.

## The run: `python3 harness.py agent.py`, once per turn

cwd `/root`, the image's environment, stdin one JSON object
`{input, tools, resources}`, stdout and stderr captured whole, no
timeout. The feed is written and the process waited for together;
the harness reads stdin to its end before it writes anything.

The harness: parse the source; if the last statement is a bare
expression, split it off, `exec` the rest and `eval` it; else `exec`
the whole. One globals dict, `__name__ == "__main__"`, with three
names the script did not define — `input`, `tools`, `resources`.
The script's prints go to a StringIO. The LAST line of the real
stdout is the envelope `{"eval": <value or null>, "stdout":
<captured>}`, then `flush` and `os._exit(0)` so nothing the script
left behind can print after it. No safeguards: rebinding `json`,
swapping `sys.stdout` back, or `sys.exit(0)` breaks the envelope,
and that is the author's problem. No base64 anywhere — the source
is a file and the input is stdin.

The exit classifies, in the CLI's order:

- nonzero status, or a signal → `exception {status, stderr}`, the
  traceback raw;
- clean exit, last non-empty line not an envelope → `harness {line,
  error}`;
- `eval` not `null` → the value; else the printed text, trimmed,
  read as JSON → `printed` if it is not JSON; nothing printed →
  `no_output`;
- the value not a list of chunks → `deserialize {path, error}`;
- a chunk the script may not say → `forbidden {index, chunk}`.

All under `{kind: "python", error: {…}}`.

## What the script may say

A JSON array of `AgenticLoopChunk`s, and only the six assistant
kinds — `assistant_reasoning`, `assistant_text_content`,
`assistant_image_content`, `assistant_audio_content`,
`assistant_tool_call`, `assistant_refusal` — plus `notification`.
Never `user`, `usage` or `tool_response`: the delivered messages,
the accounting and the tools' answers are the container's to say.
One forbidden chunk refuses the whole output. An empty array is a
turn that said nothing: no rest, and the queue's last look. The
script may return its chunks (last expression) or print them; the
return wins when it is anything but `None`.

## What the script is fed

`input` is the history in its STORED form — the same untagged array
the continuation row holds: chunk objects, and each user message as
an array of MCP content blocks (text, image, audio, embedded
resource, resource link), this turn's message (or the delivered
messages) the last items, so a script that looks at `input[-1]` knows
which turn it is on. Nothing is converted for the script: what it
makes of an image is its own. `tools`
and `resources` are rmcp's `Tool`s and `Resource`s, RE-LISTED before
every invocation of the script — the first, and each one after a
turn's tool responses or a prompt taken at the turn's end. Nothing
is cached across turns.

## The calls: the last segment, in parallel, checked first

The last segment is everything after the most recent tool response;
since the script cannot say a tool response, the last segment is its
whole output. Every `assistant_tool_call` in it is a call,
fragments coalesced by id. Before anything of the turn is yielded,
every call's name is looked up in the very list the script was fed:
a name it does not carry is `{kind: "loop", error: {kind:
"unknown_tool", id, name}}` and ends the run — a real `500` on the
first turn, a fatal notification after. Then every call goes out at
once, each on its own task, BEFORE the turn's chunks are yielded, so
the calls run while the answer is read; each response is yielded and
recorded in completion order. Arguments that are not a JSON object
go as none; the tool's refusal is a tool response the script reads
next turn. A transport failure is fatal. No in-process reach back
into the container (no `objectiveai.execute`): the script calls
tools only by emitting chunks, and reads the answers on its next
run.

## The seams, the history, the queue

openrouter's, unchanged: after a turn's responses `QUEUE.take()`
delivers pending messages as `user` chunks and `Prompt` items, then
the history rests; a call-less turn that spoke rests first, then
`take_or_close()` — empty closes and ends the run, pending opens
another turn. The history keeps the assistant chunks with `_meta`
stripped and fragments coalesced, and the tool responses; not
notifications. One row, `continuation(id = 1, state jsonb)`.

## No usage, no timeouts

Nothing is sampled, so no `usage` chunk is ever produced. Nothing
here times anything out: not pip, not the script, not a tool call.
