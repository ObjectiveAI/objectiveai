# The lone harness — settled decisions

The operating rules for this container's implementation, accumulated
as they are settled. Each entry is a decision already made. Add to
this file as more is figured out. The execution contract is the
Python harness's (`diverge-agents/python/HARNESS.md`), carried to a
model that runs on the host instead of a script: the feed, the
envelope, the classification of a turn and the chunks it may say are
unchanged. What differs is where the turn comes from.

## What is built

The whole container, never run as an image: the agent vocabulary
with its schema (`agent/`); the server (`main.rs`, openrouter's);
openrouter's claim, queue, continuation and history verbatim;
registration as the source check (`registration.rs`); the turn
(`run/`): the feed onto `lone harness`'s stdin, the envelope off its
last stdout line; and the loop (`loop/`): list, run, check, call,
seam. Nothing is carried from `lone`: the image installs it.

## The agent

`{bundle?, endpoint?, system_prompt?, max_new_tokens?, temperature?,
top_p?, top_k?, thinking?, dependencies}`. Exactly one of `bundle`
and `endpoint`: a `bundle` is a directory inside the container,
holding the `manifest.json` `lone convert` writes; an `endpoint` is a
LoNE `serve` base URL on the network (`host:8082`, `…/v1`, or the
full `/v1/chat/completions`). The rest are the harness's flags,
forwarded only when set; every sampling member absent is the
harness's own default (greedy, 512 new tokens, thinking on). Dropped
from the Python agent's shape: `python` and `requirements` (there is
no source and nothing to install), and `upstream`, `memory`, `disk`
as before.

## Registration is the check

`POST /register` validates the agent, then checks its one source: a
`bundle` must have `<bundle>/manifest.json` as a file; an `endpoint`
needs nothing local, because the harness only makes an HTTP request.
Nothing is written and nothing is installed — the image carries
`lone`, the bundle is mounted. Both or neither source is `400` with
`kind: "source"`; a path with no manifest is `400` with
`kind: "bundle"` naming the directory; a second registration is
`409`. The whole handler runs under one lock, so two registrations
cannot interleave. Nothing of the proxy's is touched.

## Secrets: none declared, both calls served

The registration's answer carries `secrets: {}`: the model is on
disk, so no credential, and the harness is handed nothing but the
conversation and the caller's lists. The two calls of the inside
contract are served anyway (`secrets.rs`, the common port): `POST
/secrets` holds the fed values whole, replacing the last feed, and
answers `204`; `POST /secrets/collect` answers every name asked with
the value it was fed, a name never fed `500 {kind: "secrets"}` naming
the name and never a value. Declaring none, the proxy feeds nothing
and collects nothing; the handlers are the contract, not a path.

## The run: `lone harness …`, once per turn

cwd `/root`, the image's environment, stdin one JSON object
`{input, tools, resources}`, stdout and stderr captured whole, no
timeout. The feed is written and the process waited for together; the
harness reads stdin to its end before it writes anything.

The invocation is `lone harness --bundle <bundle>` (or
`--endpoint <url>`) plus one flag per member the caller set:
`--system`, `--max-new-tokens`, `--temperature`, `--top-p`,
`--top-k`, `--thinking` or `--no-thinking`. With a bundle the harness
loads it once per process, which is the whole cost of the turn on a
cold start; with an endpoint the process only makes the HTTP request,
and the bundle's load is the LoNE host's. The container keeps
nothing between turns either way, and that is deliberate — the loop
owns the conversation.

The exit classifies, in the CLI's order:

- nonzero status, or a signal → `exception {status, stderr}`, the
  reason raw on stderr;
- clean exit, last non-empty line not an envelope → `harness {line,
  error}`;
- `eval` not `null` → the value; else the printed text, trimmed,
  read as JSON → `printed` if it is not JSON; nothing printed →
  `no_output`;
- the value not a list of chunks → `deserialize {path, error}`;
- a chunk the harness may not say → `forbidden {index, chunk}`.

All under `{kind: "lone", error: {…}}`.

## What the harness may say

A JSON array of `AgenticLoopChunk`s, and only the six assistant
kinds — `assistant_reasoning`, `assistant_text_content`,
`assistant_image_content`, `assistant_audio_content`,
`assistant_tool_call`, `assistant_refusal` — plus `notification`.
Never `user`, `usage` or `tool_response`: the delivered messages, the
accounting and the tools' answers are the container's to say. Nor
`user_structured_decisions` or `structured_decisions_response`: this
agent makes no structured decisions. One forbidden chunk refuses the
whole output. An empty array is a turn that said nothing: no rest,
and the queue's last look.

This harness is text-only, so it says reasoning, text and tool calls.
The last two come from the generated text: the turn is split at
`</think` into reasoning and content, and both LFM grammars are
parsed back into `assistant_tool_call` chunks with JSON-string
arguments. LoNE being text-only is the model's, not this image's:
the kinds it never emits are still legal here.

## What the harness is fed

`input` is the history in its STORED form — the same untagged array
the continuation row holds: chunk objects, and each user message as
an array of MCP content blocks (text, image, audio, embedded
resource, resource link), this turn's message the last item.
`tools` and `resources` are rmcp's `Tool`s and `Resource`s,
RE-LISTED before every invocation of the harness. Nothing is cached
across turns.

The harness re-renders that history through the bundle's own chat
template. Image and audio blocks are marked as text (`[image]`,
`[audio]`), never dropped silently; a resource's text is its own; a
tool response is a `tool` message; assistant fragments coalesce by
id. What the template does with that is the bundle's.

## The calls: the last segment, in parallel, checked first

The last segment is everything after the most recent tool response;
since the harness cannot say a tool response, the last segment is its
whole output. Every `assistant_tool_call` in it is a call, fragments
coalesced by id. Before anything of the turn is yielded, every call's
name is looked up in the very list the harness was fed: a name it
does not carry is `{kind: "loop", error: {kind: "unknown_tool", id,
name}}` and ends the run — a real `500` on the first turn, a fatal
notification after. Then every call goes out at once, each on its own
task, BEFORE the turn's chunks are yielded. A transport failure is
fatal. No in-process reach back into the container: the harness calls
tools only by emitting chunks, and reads the answers on its next run.

## The seams, the history, the queue

openrouter's, unchanged: after a turn's responses `QUEUE.take()`
delivers pending messages as `user` chunks and `Prompt` items, then
the history rests; a call-less turn that spoke rests first, then
`take_or_close()` — empty closes and ends the run, pending opens
another turn. The history keeps the assistant chunks with `_meta`
stripped and fragments coalesced, and the tool responses; not
notifications. One row, `continuation(id = 1, state jsonb)`.

## No usage, no timeouts

Nothing is sampled upstream, so no `usage` chunk is ever produced.
Nothing here times anything out: not the bundle load, not a turn, not
a tool call.

## The host: a bundle wants macOS, an endpoint wants a server

`lone` targets Core ML and the Apple Neural Engine, so a **bundle**
turn needs macOS. The image installs the harness **without** Core ML:
`coremltools` publishes no manylinux wheel for arm64, and installing
it would make the image unbuildable on the machine most likely to
build it. So `lone --help` works anywhere and the container registers
anywhere; a bundle turn on a Linux provider exits with `lone targets
the Apple Neural Engine and needs macOS` on stderr, which the loop
reports as the turn raising.

An **endpoint** turn is a thin client: `lone harness --endpoint URL`
posts the conversation and the tool definitions to a LoNE
`serve --bundle … --host 0.0.0.0` on the network, and nothing is
loaded in the container. That is the mode a Linux podman provider
runs: no Core ML, no ANE, no mount. The LoNE server holds the bundle
and does the inference; `serve` renders the OpenAI `tools` field
through the bundle's template and answers with `reasoning_content`
and `tool_calls`, which the harness parses back into the same chunks
as a local turn.

LoNE repository: [LoNE #3](https://github.com/AJ-Gonzalez/LoNE/issues/3),
implemented in LoNE #4.
