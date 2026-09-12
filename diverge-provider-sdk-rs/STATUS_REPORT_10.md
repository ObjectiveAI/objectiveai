# Provider Protocol — Status Report 10

Report 9 ended with one hole: the loop. This period filled it. The
OpenRouter container is functionally complete — request in, agentic
loop run, tool calls through the proxy, continuation token out — and
has a Containerfile that builds it. The ClaudeCode container is well
underway on a very different foundation: no SDK at all, but Claude
Code's own on-disk conversation state, researched from unminified
source and reified as the continuation format itself.

## Tool calls became chunkable

The first move was structural, in the SDK, and breaking. The
assistant tool call chunk was divorced from MCP's
`CallToolRequestParams` and unflattened into bare fields of its own:
`id`, `name`, and `arguments` as an **`Option<String>`** — not a JSON
object but a streamed fragment of JSON text, concatenated across
chunks sharing an `id`. A model that streams its arguments now maps
onto the wire without buffering; fragments of interleaved calls
correlate by `id`, not adjacency. The MRTR retry fields left in the
same stroke — retry bookkeeping was the old world's concern. The
specification's `assistant_tool_call` page declares the bare fields,
and the openrouter crate followed.

With that, the SDK's `response::push` earned its final form: one
chunk pushed onto a history at a time; the streamed text kinds
(reasoning, text content, refusal) coalescing by adjacency; tool call
fragments coalescing **by id, searched backwards, stopping at the
first tool response** — the chain since the last answer is the only
place a fragment's parent can be. (This one took a revert and a
reapply to find its shape; the wire was right before the API was.)

## The OpenRouter loop, complete

- **`fetch`** — one OpenRouter call as a chunk stream. The old api
  crate's SSE handling ported with every runtime fix intact:
  `[DONE]`, comment and empty-data frames, `serde_path_to_error`, the
  provider-error fallback parse, the first-item contract. The API key
  is an argument, never Bearer-prefixed by us. The api crate's
  `StreamOnce` came along so the returned stream is honestly `Unpin`.
- **`into_chunks`** — the upstream chunk fanned into the response
  vocabulary in fixed order (reasoning → content → images → refusal →
  tool calls), each produced chunk stamped with `_meta` provenance:
  `{"openrouter": {"id": …}}`.
- **`r#loop`** — the loop itself. It connects to the proxy at its
  hard-coded loopback address, and then, per turn: list tools (fresh
  EVERY turn — the caller's tool set may change while tools run),
  fetch, and drain — **yield first, remember second**: every chunk
  goes out the moment it arrives and is also accumulated into the
  running history via `push`, `_meta` stripped, because the history
  keeps what was said, not where it came from or how it was cut.
  After a clean turn, every tool call runs **in parallel**, each
  response yielded and recorded the instant its own call completes,
  in completion order. No calls means the model is done, and the
  loop's last word is the whole history tokenized — the continuation
  chunk is the final yield. Any yielded error ends the stream,
  permanently.
- **`serve`** — the loop wired into `POST /`, with the dictated error
  taxonomy: wrong agent and unparseable continuation are the
  caller's fault, a missing `OPENROUTER_API_KEY` is the server's,
  OpenRouter's status is inherited, and a mid-stream error becomes a
  fatal notification chunk.
- Tool schemas are requested **strict**; a call whose accumulated
  arguments never became a JSON object is sent with none, and the
  tool's refusal comes back as content for the model to read.

The proxy learned the counterpart law: `call_tool` **never** returns
`Err(ErrorData)`. Every tool-level failure — the far side's refusal,
the relay's unanswered or unreadable — is folded into a
`CallToolResult::error` the agent can read and react to. An `Err`
out of `call_tool` is thereby reserved for the MCP link itself
failing, and the loop on the other side depends on exactly that
distinction: tool failures are conversation, link death is fatal.

## The first Containerfile

`diverge-agentic-loop-openrouter/Containerfile`, built from the
workspace root so the path dependencies ride along:
`rust:1.98.0-alpine3.24` builds both binaries as musl (digest-pinned,
researched — never from memory), the final stage is `alpine` by
digest, and the entrypoint backgrounds the proxy and execs the loop —
two programs, one container, ports 8080 and 8081 as the Container
specification says. The workspace lockfile is copied into each crate
so resolved versions hold without `--locked`. The known races — the
loop dialing 8081 before the proxy binds, the provider dialing 8080 —
were examined and deliberately accepted.

## The ClaudeCode container: the token IS a filesystem

`diverge-agentic-loop-cc` uses no SDK. Claude Code persists its
conversation as append-only JSONL under
`~/.claude/projects/<sanitized-cwd>/<session>.jsonl` with adjacent
session-keyed families — no locks, no indexes, no daemons. That fact,
established from a clone of unminified source (`temp/`, untracked)
and recorded in `cc-source-findings/HISTORY.md` — strictly
evidence-based, file:line cites, no gaps filled — is the whole
strategy: **the continuation token carries the session id and the
files**. Running a continuation means writing them back before Claude
Code starts; ending a run means harvesting them.

`continuation.rs` implements both directions in one impl block, tokio
fs only:

- **`parse` / `tokenize`** — base64 over JSON, same coat as
  openrouter's.
- **`write`** — every file laid down under the hard-coded
  `CONFIG_DIR = "/root/.claude"` (the stock default; the container's
  environment stays unmodified), parents created as needed; a path
  that is absolute or contains `..` is refused — the token does not
  get to escape the config directory.
- **`read`** — the harvest, keyed by the session id Claude Code's own
  stream-json output reports: the transcript found **by name** in
  every project directory rather than recomputing the cwd sanitizer;
  the session's directory subtree (subagents, tool results, session
  memory); `file-history/<session>/**`; `tasks/<session>/**`.
  Deliberately not harvested: the global prompt history, binary
  caches, debug output, config. Absence is not an error; the walk was
  re-verified against source after writing.

The cc Containerfile mirrors the openrouter one and bakes in
`@anthropic-ai/claude-code` at a pinned version on Node. A detour
into privilege-splitting the spawned agent from the harness (a
dedicated agent user) was built and then deliberately undone —
everything runs as root, and the config dir stayed `/root/.claude`.

## Research: what shapes a Claude Code agent

With storage solved, the open design question is the cc **agent
parameter surface**, and the period closed researching what Claude
Code can be given in programmatic mode, from source:

- **Plugins are the unit.** One directory — optional manifest,
  `commands/`, `agents/`, `skills/`, `output-styles/`,
  `hooks/hooks.json`, MCP config — every component optional. A
  caller-supplied plugin carries all agent furniture at once.
- **Installation has a spectrum**, and the far end is ours:
  `claude plugin install` with marketplaces at one end; the headless
  reconciler that print mode runs at startup; a seed-directory env
  var for pre-baked caches; and **`--plugin-dir`** — session-only
  plugins loaded straight from a directory, no network, no
  registration, overriding installed ones by name. Materialize a
  directory, add a flag: the same move `write` already makes.
- **Memory has a root-level tier.** `~/.claude/CLAUDE.md` — in this
  container, literally `/root/.claude/CLAUDE.md` — loads regardless
  of cwd in stock CLI print mode, before the per-directory walk. A
  caller-supplied memory blob is one more file laid down before
  launch, and the harvest correctly ignores it: agent furniture and
  conversation state stay separated.

## What remains

- **The cc harness itself** — `main.rs`: serve on 8080, write the
  continuation, launch `claude -p --resume` with stream-json,
  translate its output into the chunk vocabulary, harvest, tokenize.
- **The cc agent surface** — what the request carries (system prompt,
  plugins, memory, permission flags), informed by the research above;
  the SDK's claude_code agent likely grows fields.
- **Images, live** — neither Containerfile has been built and run;
  the image references and resource ceilings in SDK and specification
  are still stated constants.
- **The remaining spec endpoints**, in tag order, when attention
  returns to the site.
