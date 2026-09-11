# Provider Protocol — Status Report 11

Report 10 left the ClaudeCode container with storage solved and a
harness to build. This period built nearly all of it — and, on the
way, gave the protocol two new capabilities every agentic-loop
container shares: a way to fetch agent furniture the server is
missing, and a queue for steering a conversation already running.
The cc container now has a typed model of everything Claude Code can
say, a conversion of that wire into the chunk vocabulary, a
subprocess harness with strict message fates, and live queue
endpoints. What remains of it is one function: the root handler.

## The cc agent surface, and fetching what it names

The claude_code agent grew its furniture fields: **`skills`** — a
map of skill name to **dirhash** — and **`claude_code_agents`** — a
bare list of dirhashes, because a subagent's name is its filename
and travels with the file. The dirhash (a deterministic recursive
directory hash, the golang module-hash lineage) is the identity of a
skill or agent directory: content-addressed, so a server can know
what it already has.

Mounting them is a PROTOCOL obligation, not an implementation
choice: a server MUST mount every skill and agent the request
names — the request naming a dirhash is the requirement that the
directory be present in the container. For directories a server
does not already hold, the SDK gained **`fetch`** — a server-sent
channel exchange asking the client for one by `(kind, dirhash)`.
The answer is one file per frame —
`[u32 length][JSON path array][raw bytes]` — and a bare finish with
no frames means the client does not have it either: absence is an
answer, not an error. The exchange is endpoint-local, not shared —
nothing else speaks it. The client side (`fetch_proxy`) is wired.

## Everything Claude Code can say, typed

The stream-json stdout vocabulary was researched from unminified
source and reified as the cc crate's `response` module: the whole
`StdoutMessage` union — sixteen record kinds, sixteen system
subtypes, both result arms, the full control vocabulary, fifteen
assistant content-block kinds, the request-side param blocks —
**strict**: a line that matches no known record fails the parse,
because the image pins Claude Code's version and the union is closed
by construction.

Four difference audits (`reports/1.md`–`4.md` in the cc crate)
ground the module in a doctrine now standing crate-wide: **the
emitter is truth** — the source's zod schemas are declarations never
run as validators, so the emission sites define the wire, and where
they disagree with the schema, the type admits both. Untagged unions
with child-carried literal markers everywhere; flat open enums
(named knowns plus a verbatim `Other(String)` tail) for API-owned
vocabularies; deserialize-only, no `Serialize`, no
`skip_serializing_if`; **no serde defaults** — every tolerated
absence is an `Option`, because absent and zero are different facts.
The newer-SDK block surface was ported from the api crate's own
claude_agent_sdk module, which was audited both ways and mined for
what it solved (report 4): its shapes taken, its closed vocabularies
and silent line-drops deliberately not.

## The queue: steering a running conversation

A new protocol feature, defined in the SDK for every agentic-loop
container. Two client-sent channel requests:

- **`enqueue`** `{prompt}` — the response IS the message's fate, and
  it arrives only when the fate is known, with **no timeout
  anywhere**: `delivered` (it entered the conversation; the new
  **`user` chunk**, carrying the prompt verbatim, marks the position
  where), `dequeued` (the caller withdrew it), or `missed` (the run
  ended first; nothing malfunctioned — a caller that still wants it
  heard sends it as the next run's prompt).
- **`dequeue`** — clears the whole queue; answers `dequeued` or
  `empty`, while each withdrawn message's own enqueue answers
  `dequeued` itself.

The root `agentic_loop_container` module gives containers their
shared shape: request/response aliases, the queue verbs' bodies,
`POST /enqueue` and `POST /dequeue` beside `POST /` on 8080 — and
**no in-band error variants**: a container that cannot answer says
so as HTTP does.

OpenRouter implements it as the reference: a container is one run
(an atomic CLAIMED swap; the second request gets `409`), delivery at
the loop's two seams — folded onto the tool-response turn inside one
`<system-reminder>` section derived at message-construction time
only (the continuation stores bare prompts), or opening a new
turn — and categorical fate delivery: the queue closes under the
same lock hold that proves it empty, a drop guard covers unpolled
streams and panics, and the enqueue window is maximized by
tokenizing speculatively before the last look.

## The cc harness: Claude Code holds the queue, we hold the fates

cc does not own its loop, so its queue machinery is a bridge to
Claude Code's own: an enqueue becomes a stream-json stdin `user`
message with a minted uuid; a dequeue becomes one
`cancel_async_message` control request per queued uuid; and the
fates are read back off stdout.

The architecture iterated hard — a writer task behind a command
channel was built and eradicated; a single session lock replaced it;
and it settled as **two mutexes and a map**: `WRITER` (stdin —
holding it is the sole right to write AND to register a fate),
`REPLIES` (the cancel-reply receiver; the reader task permanently
holds the sender, so reading replies is a lock holder's right), and
`PENDING` (uuid → oneshot fate wire, beside the locks, never under
them — its keys ARE the queue as the container knows it). A fourth
global — an explicit FIFO gate for the withdrawal boundary — was
added and then deleted on the realization that **the fair writer
mutex is itself the gate**: a dequeue's position in the writer's
wait queue is the boundary, everything ahead registers and is
withdrawn, everything behind is out of reach — and the explicit gate
could not have joined the parallel lock acquisition without a
genuine deadlock against enqueue's gate-then-writer hold.

Fates are **strict** — answered when truly known, not when the
write lands: the replay echo (`--replay-user-messages`) resolves
`delivered` and pushes the `user` chunk at exactly that stream
position (Claude Code yields tool responses before the replay, so
the mark lands behind the answers it followed); the dequeue reads
Claude Code's control responses under the lock, each reply's
`cancelled` flag deciding that message's fate — `true` is
`dequeued`, `false` means the queue no longer held it and its true
fate is `delivered`; a failed write, a dead wire, or the reader's
end-of-stream drain answer `missed`, and the drain runs only after
the writer is cleared, so nothing can register behind it. The
standing deadlock audit: the reader takes no lock while reading —
only at end of stream, after dropping the reply sender, one lock at
a time — so stdout always drains while an enqueue blocks on a full
stdin pipe.

The queue endpoints are LIVE in `main.rs`; `POST /` answers an
honest `501`.

## The conversion, and the positional error policy

`into_chunks(self, chunks: &mut Vec<AgenticLoopChunk>)` on every
response type, in openrouter's conventions: parents delegate each
child field to the child's own method, chunk literals only at
leaves, rmcp inners via their constructors. The doctrine:

- **Usage from the `result` record ONLY** — the api crate's law,
  verified at its source and kept: nothing accumulates, per-message
  usage is never read, prompt tokens are input plus both cache
  sides, one usage chunk per run, emitted for both result arms.
- All three tool-use kinds (client, server, MCP-connector) emit
  tool-call chunks with their JSON arguments; tool results answer
  as tool-response chunks with faithful text and image content;
  subagent narration is skipped (the sidechain's outcome arrives as
  the spawning call's tool result); every silent record kind is
  enumerated with its reason.

Error-typed records — a **rejected** rate limit, a **failed** auth
status (`--enable-auth-status` now rides the argv), an error
result — are judged by POSITION: before the first assistant message
they are the request's own failure and travel as the stream's `Err`
the moment they are seen, chunks forfeited, exactly as openrouter's
pre-stream failures emit nothing; after it they are news inside a
working run — the rate limit a NON-fatal notification, because the
automatic retry is Claude Code's own queueing; the failed auth
fatal; the error result its fatal notification plus the bill.
Deliberately not error-typed: assistant `error` markers and
`api_retry` narration — Claude Code's retry in flight, whose
terminal verdict arrives as the result record. The reader keeps
draining after any `Err`; aborting is the consumer's choice.

The reader's sender now carries the converted stream:
`Result<AgenticLoopChunk, Error>`, the `Error` payloads whole
records for the root handler to turn into status and body.

## What remains

- **The cc root handler** — the one unbuilt function: CLAIMED,
  agent/continuation validation, the `spawn()` call, SSE framing,
  `Error` → HTTP mapping, and at exit the `Continuation::read`
  harvest → tokenize → continuation chunk. Everything it needs now
  exists.
- **SDK wiring** — the client executor for the queue and fetch
  channels; the server handle's registration for the same.
- **Images, live** — neither Containerfile has been built and run;
  the cc Containerfile still predates the queue-era argv.
- **Spec pages** — the queue verbs, the `user` chunk, `fetch`, and
  the cc agent's furniture fields await prose.
- **Deferred by decision** — `--include-partial-messages`,
  rich-content prompts, `_meta` provenance on cc chunks.

Two crate-local status trails now exist: this series for the
protocol, and `diverge-agentic-loop-cc/reports/` (1–4 the wire
audits, 5 the container status) for the cc deep detail.
