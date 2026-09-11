# Provider Protocol — Status Report 9

Report 8 ended by naming the next work: the first containers. This
period built them — or rather, built the three crates they are made
of, a new wire for one of them to speak, and everything an OpenRouter
run needs except the loop itself. For the first time since this
project began, there is code on BOTH sides of the protocol's socket.

## Three new crates

- **`diverge-agentic-loop-mcp-proxy`** — the MCP proxy that rides
  inside every agentic_loop container. Implemented, and described
  below.
- **`diverge-agentic-loop-openrouter`** — the OpenRouter container.
  Scaffolded end to end; the loop itself is the one hole left.
- **`diverge-agentic-loop-cc`** — the ClaudeCode container.
  Initialized and waiting its turn.

All three are workspace members, versioned by `version.sh`, at 2.3.0,
`publish = false` — they ship as images, not to a registry.

## The proxy wire: `mcp_proxy` in the SDK

The SDK gained a root module defining the wire between a provider
server and the proxy inside a container: WebSocket BINARY frames, the
channel discipline in miniature. The two directions frame differently
because they have different amounts to say — the container sends one
kind of frame, so it spends no type byte (`[channel: u8][request…]`,
a struct, not an enum of one); the server sends two, so one byte says
which (`0` response, `1` finish). The container's request is TYPED —
the same `channel_request::Frame` a server's own channel request
carries on the main wire — and the server's responses stay bytes,
because which exchange a response answers is the opener's knowledge.

Channels are `u8` tags, minted by the container, unique among live
channels, reusable after a finish. One connection at a time; a second
is refused; a dead one is waited out.

And the law that shaped everything after: **a connection dying does
not fail an exchange.** An exchange is answered only when its
response has arrived AND its channel has finished; anything less is
un-answered, and the container asks again on the next connection —
forever, because the agent inside is waiting and nothing in the
container has the standing to tell it no. A server may therefore see
the same logical ask as two wire exchanges, and an ask with side
effects may run twice; that is the chosen trade, stated in the module
doc. The one non-answer never re-asked is the deliberate one: an
empty finish is a statement, not an accident.

## The proxy itself

To the agent beside it, a fully compliant MCP server on loopback at
`/mcp` — rmcp's Streamable HTTP server with a custom handler, every
method a relay. To the provider outside, a WebSocket listener at `/`
on the Container section's port 8081. One binary, one listener, both
paths.

- **The connection slot.** Claim-before-upgrade, generation-counted,
  released on drop — a failed upgrade cannot wedge the slot shut, and
  a second arrival gets `409` and no socket.
- **Generation-stamped routes.** The channel table is stamped with
  the generation it serves; registration checks the stamp under the
  same lock, and release wipes only its own generation's table. This
  closed the period's two review-found bugs before anything ran: a
  route registered after its connection's wipe (a channel nothing
  would ever end — an agent hung forever), and a stale wipe killing a
  newer connection's live routes (tags freed while the far side still
  held them — misrouted answers).
- **Ask, until answered.** The unary exchanges ride the retry law
  above. The failures an agent can still see are the incurable ones:
  the far side's deliberate refusal, an unreadable answer, params
  that would not serialize.
- **One resident notifications stream.** Not per-session: the proxy
  maintains a single notifications channel for its whole life —
  re-opened immediately when a connection dies, re-opened on the NEXT
  connection when the far side ends the stream deliberately — and
  every notification is shipped, once, to every MCP session the proxy
  serves. No queueing, no replay at this layer; a session leaves the
  registry only by failing a send. rmcp's own session cache handles
  SSE reconnect replay beneath us.
- **Honest advertisement.** `tools` and `resources` with
  `listChanged: true` — a client that was not promised a notification
  is entitled to ignore it — and the server introduces itself as
  **`diverge`**, at the crate's own version, rather than rmcp's
  default of introducing every server as "rmcp".
- **No timeouts.** Nothing in the crate measures time; the only waits
  are for the next connection and the next frame.

## The OpenRouter container

`POST /` on the Container section's port 8080 takes the SDK's own
request type as its JSON body; the answer is an SSE stream, each
event one chunk of the response vocabulary. The `chunks()` between
them — the loop — is the one `unimplemented!()` left.

Around that hole, the period built the whole conversion story:

- **`upstream`** — the OpenRouter API as types, ported faithfully
  from the old api crate with every `objectiveai_sdk` type its fields
  referenced inlined (~1,400 lines, self-contained), then pruned to
  what the container can actually derive: `response_format`,
  `prediction`, `seed`, `tool_choice`, `parallel_tool_calls` and the
  request-sourced provider routing fields are gone, because nothing
  in the provider request carries them.
- **Messages, one file per role** — `system`, `developer`, `user`,
  `assistant`, `tool`, flattened into one `role`-tagged enum with
  unchanged wire forms, each role with its own constructor. The
  folding law: **consecutive assistant chunks always merge into one
  assistant message** — content kept as parts in arrival order,
  reasoning and refusal concatenating, tool calls accumulating — and
  a tool response, or the next prompt, is what ends a run of them.
- **Content conversions** — every MCP content block becomes the
  `RichContentPart` it is: text as text; images and audio as base64
  data URLs and format tokens (`mime2ext`, so `audio/mpeg` is `mp3`);
  embedded resources dispatched on their mime, video included —
  MCP's content blocks have no video, but a resource's bytes can be
  anything, so `InputVideo` earned its place back; a resource link is
  a file by URL, fetched by nobody.
- **The continuation token, designed** — base64 over a JSON array of
  untagged items, each either a chunk the loop produced or a turn's
  user prompt (an object and an array cannot collide). That second
  arm is what carries the conversation across turns: the wire's
  chunks never contain the caller's prompts, so the token must.
- **`ChatCompletionCreateParams::new(agent, continuation, prompt)`**
  — every agent parameter moves across through per-type `From`
  conversions, and the messages assemble in exactly the order the
  conversation happened: system prompt → history → this turn's
  prompt. `tools` alone stays `None`, the loop's to fill from the
  proxy's `tools/list`.

## SDK and specification, alongside

- The openrouter agent gained **`system_prompt`** — a plain optional
  string, sent as the conversation's leading message. A breaking
  addition at 2.3.0, reflected in the specification the same commit
  cycle. It quietly retires the last of the "post-transform"
  doctrine: the system prompt now travels IN the request, because the
  container is the one that must emit it.
- The specification site holds at 39 pages, green, with the
  openrouter agent page carrying the new field.

## What remains

- **The loop.** `chunks()` — the OpenRouter call, the SSE decode into
  the chunk vocabulary, tool calls out through the proxy client,
  `tools` from `tools/list`, and the continuation token minted on the
  way out. Everything it needs is now on the shelf beside it.
- **The ClaudeCode container**, after the OpenRouter one proves the
  shape.
- **Images.** Dockerfiles for the three binaries, and with them the
  image references and resource ceilings the SDK and specification
  still carry as stated constants.
- **Live proof.** Still nothing executed end to end — but the period
  ended with both halves of the proxy wire implemented, which is the
  first time a diverge socket has had a program on each end.
