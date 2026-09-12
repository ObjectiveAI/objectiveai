# Provider Protocol — Status Report 12

Report 11 left the ClaudeCode container one function short of
running. This period finished that function, then went much further
than any period before it: the protocol gained mounts, an
environment, resources, a raw-bytes continuation, a WebSocket
container surface, thread attribution on chunks and a frame for
handing rotated state back; the workspace gained three container
crates; and one of them — Hermes — went from an empty skeleton to a
feature-complete harness of some 4,700 lines, with the research
that justifies every decision written down beside it. This report is
assembled from the 123 commits since report 11 rather than from
memory, so that nothing is left out. It is long because the period
was.

## The cc container, finished

The root handler landed: `POST /` (later `GET /`, see the WebSocket
section) claims the container, judges the agent kind and the prompt,
parses the continuation, spawns Claude Code and streams the run.
`spawn()` now returns the run AS A STREAM — the reader is driven by
whoever polls, not by a task — with the agent's knobs on the argv.

Two error disciplines were settled and stayed for every container
since. **Fatality is finality**: an error record mid-stream is HELD,
not sent; a later chunk proves the run outlived it and flushes it
as a non-fatal notification, in arrival order; when the stream ends
with errors held, only the LAST is the run's death, fatal, and the
ones before it flush non-fatal ahead of it. **The salvage**: when a
run dies after the model has spoken, the harvest still follows the
fatal word — progress worth resuming closes the run — and a run that
died unspoken saved nothing beyond what the caller brought. The
session id is captured off the wire (`spawn/session_id.rs`) so the
harvest is keyed by the session Claude Code actually ran.

The image moved to Debian sid, because skills carry scripts and
scripts need a real distribution; and it ships NO Claude Code —
licensing — so `spawn/install.rs` installs the pinned package at
startup, memoized once, and every endpoint gates on that outcome.
The cc-source-findings and reports folders were dropped once their
content had moved into code docs.

## The openrouter container: the watermark, three times

The salvage arrived here too — the completed history follows a
fatal error as the closer — and its watermark took three fixes to
get right: it advances on call-less turns that said something; the
tool-seam prompts ride BELOW it (they fold onto the tool responses
at request-building time, a valid resume); and an empty turn is no
resting place, because its opening prompt hangs unanswered.

## Three container crates bootstrapped

`diverge-agentic-loop-hermes`, `diverge-agentic-loop-pi` and
`diverge-agentic-loop-eliza` joined the workspace and `version.sh`
— skeletons serving the container surface and refusing the run,
each with a gitignored `sources/` for research clones. Eliza got its
first report (where its documentation lives); pi stays a skeleton.
Hermes is the rest of this report.

## The protocol: what the request can carry

- **Mounts.** The one `fetch` exchange split into `fetch_file` and
  `fetch_directory`; the cc agent's `skills` and `agents` fields
  left, replaced on the REQUEST by `file_mounts` and
  `directory_mounts` — absolute container path to a size-bearing
  identity (`f1:<size>:<b64url sha256>` for files, `d1:` over a
  manifest for directories), mounted read-only before the container
  starts. Skills and agent definitions are now whatever the caller
  mounts where the upstream reads them.
- **Environment.** The request carries `environment`, the
  laboratories' way: set on the container before it starts, the
  server's business, never the harness's — and it carries neither
  kind of credential (a rule written into the docs).
- **The channel-request table** now reads: 0 `McpListTools`,
  1 `McpListResources`, 2 `McpCallTool`, 3 `McpReadResource`,
  4 `McpNotifications`, 5 `FetchFile`, 6 `FetchDirectory`,
  7 `FetchResource`, 8 `FetchContinuation`. The client's
  `FetchProxy` grew to four methods, and `client/mod.rs` and the
  server handle followed.

## The queue lives in the MCP proxy

`enqueue`/`dequeue` landed in `diverge-agentic-loop-mcp-proxy`: a
pending prompt is folded onto the next tool response, at its HEAD,
as one `<system-reminder>` section, at the exact moment that
response goes to the agent — after the retry law, so a re-asked call
folds once. The proxy's `/enqueue` blocks until then and answers
`{"attached": <sha>}`: SHA-256, lowercase hex, over the folded
result's text blocks concatenated with no separators — the caller's
correlation key, computed upstream of every transform the harness
applies. `/dequeue` withdraws everything pending. The fold was moved
to the response's TAIL for one day and reverted to the head.

## The hermes agent, in the SDK

The `hermes` agent arrived with a provider, an effort ladder,
toolsets and an api mode, then was cut to what Hermes can honorably
be told: `api_mode` left (the profile knows its dialect),
`base_url`/`max_turns`/`run_budget_seconds` left, `auto` left (a
request names its provider). The effort ladder is Hermes's own
eight rungs, `none` to `ultra`, clamped to each wire's subset at the
nearest weaker rung.

**Toolsets** went tri-state, then narrowed to the mount-enablable,
then — after reports 4 through 9 judged the questionable ones —
lost `computer_use` (a driver present with no display shows the
model a tool that cannot work), `cronjob` (live under the gateway,
where the future does not exist), `stt` (config, not a tool),
`clarify` (a safe no-op today), `yuanbao` (coupled to a platform we
do not run), `context_engine` (an empty socket), `delegation`
(below), and `spotify` for a while. Then **toolsets became
auth-carrying structures** (report 13): arg-less ones are plain
`Option<bool>` switches on `Toolsets`; arg-bearing ones are
`Option<Toolset>` structs with the credential as an argument —
`web` as two one-of slots (search: Tavily/Exa/Parallel/Keenable/
Brave/Searxng; extract: Firecrawl), `browser` with one remote slot
(CDP url, or Browserbase key + project), `image_gen` and
`video_gen` with a REQUIRED `fal_key` (generation has no keyless
floor; the shared-env rule says two suppliers of `FAL_KEY` must
agree), `x_search` with a required key, `tts` with an optional
ElevenLabs key over the edge-tts floor, `homeassistant` with url and
token both required (mDNS is dead in a container), and `spotify`
back as a static client id plus a rotating `auth_resource`. The
bool-or-object union was authorized and then revoked as
unnecessary. `memory` was dropped on the reasoning that the
continuation is our cross-run memory — and restored once the
continuation was designed to CARRY the two memory files, which only
the memory tool writes; the earlier ruling had been circular.

**Providers** (report 10): `Provider` is an untagged union of 43
per-provider structs, one file each, auth as an ARGUMENT — 32
key-only `{provider, api_key}`, plus copilot, azure-foundry, custom,
zai, bedrock (bearer or access-key trio), vertex, opencode-free —
and four **rotating-OAuth providers as resources**: nous,
openai-codex, minimax-oauth (`auth_resource`) and qwen-oauth
(`oauth_creds_resource`). A resource is caller-held state the run
MUTATES; its field carries an identity, never bytes. The user
reversed my dropping of these four: refresh-token rotation was the
whole reason auth became arguments. `copilot-acp` alone stays
dropped.

**Delegation** left the vocabulary after two reports
(subagent-correlation, delegation-observability): on the API server
every delegation is background — the run ends before its children,
whose results wake the session as a turn nobody drives. It could be
supported through end-of-turn webhooks; the decision was not to.

## Resources, end to end

The `fetch_resource` exchange mirrors `fetch_file`: server channel
request tag 7, bare-bytes client frames chunked at `CHUNK_SIZE`.
On the container surface a resource lands on three plain routes,
the identity a PATH segment: `POST /resource/{identity}` (bytes
verbatim), `…/complete` (`{}`), `…/error` (`{error}` — for a
delivery that can never finish); completion XOR error settles the
identity, and anything after is `409 settled`. Every container
serves the three; cc and openrouter answer them `409 unrequested`.
Hermes's store shards on DashMap, and its fetcher decodes the
ASSEMBLED bytes as UTF-8, never the chunks.

And the way back: the **resource frame**. A new byte variant on the
wire's server response frame (tag 2, the continuation moving to 3)
and on the container's (tag 3, the continuation moving to 4): a u32
name length, the name — the request field's dotted path,
`provider.auth_resource` — then the body, the resource's whole new
content. Not terminal; the last one wins; the caller replaces what
it holds. The client stream yields it as `Resource { name, body }`.
The emit-back mechanism the resource design had left open is this.

## The chunk vocabulary, and the continuation as bytes

`parent_tool_call_id` rides the six assistant chunks and
`tool_response`: absent on the main thread, the spawning tool
call's id on a sub-agent's thread. cc emits its sidechains
attributed; openrouter sets none; the SDK's `push()` merge went
naive — a tool-call fragment's home is the most recent chunk with
the same id on the same thread, wherever it sits, and the text
kinds merge by adjacency gated by thread.

The `continuation` chunk is GONE. A continuation is raw bytes — no
base64, no JSON coat — fetched by the server over
`fetch_continuation` (tag 8, payload-less; an empty finish is a
fresh start, not a refusal), removed from the request, and sent back
as the CLOSER: the last frames of the response, chunked at
`CHUNK_SIZE`. The client's `ExecuteStreamItem` yields the pieces as
`Bytes` views, non-terminal. Then a rule the user set: **the chunks
are kept, not joined** — the caller stores the pieces as pieces and
replays them one frame each, in order; nobody on the way concatenates
— so a provider may put meaning in the boundaries. Hermes does.

## The container surface is a WebSocket

SSE cannot carry raw bytes, and the user chose a socket over a
plain streaming body (a wash on throughput; the socket gives message
boundaries and a close for free). `GET /` upgrades; the server's
first binary message is the wire's request frame verbatim, tag
included; every message back is one `Response` frame with
`Encode`/`Decode`, tags in run order: 0 `FetchContinuation` (the
tag is the whole ask), 1 `FetchResource` (JSON `{identity}`),
2 `Chunk`, 3 `Resource`, 4 `Continuation`. A relaying server
forwards 2, 3 and 4 by re-tagging one byte and consumes 0 and 1.
The upgrade is refused only for what is knowable before the request
(`409` claimed, cc's `500` install); after it there is no status
left, so a run that cannot start says so as a fatal `notification`
chunk. The continuation arrives at the container exactly as a
resource does — `POST /continuation`, `…/complete` (a LONE
completion is the fresh start), `…/error` — into a keyless slot; cc
and openrouter's continuations are plain JSON bytes now, base64 and
its dependency gone, and both compile again on the socket.

## Hermes: the research

Fourteen reports, numbered 1–9 and named after that (the user's
ruling): learning resources; every inference source (44 profiles,
39 plugins); streaming rides ACP (superseded — the wire became
`/v1/runs`); the six questionable toolsets; provider auth as
arguments; oauth-resources (the exact state documents, lossy
rewrites, the wizard-only qwen marker); gateway-tool-response (tool
responses are NEVER on the gateway stream — omission, not
truncation); toolset-auth; run-event-stream (the twelve events,
data-only frames, keepalive and closed as SSE comments, one
destructive subscriber, 300s TTL); parallel-tool-calls (starts and
completions in original call order — the i-th pairs with the i-th);
subagent-correlation; delegation-observability. `HARNESS.md` at the
crate root is the living decisions document: auth on the filesystem
from the request; the yolo trio (`HERMES_YOLO_MODE=1`,
`security.protected_instruction_files: false`, `elicitation:
{enabled: false}` on the proxy entry — no master switch exists;
approvals are answered `once` the instant they fire against the
300-second fail-closed stall); the wire; toolsets applied, not
passed through; resources; the continuation; the runner; the
socket; the image.

## Hermes: the continuation

Three files under the fixed `/root/.hermes`: `state.db` WHOLE
(schema v26 and moving; the gateway-only tables are empty here; the
compaction flags, the `system_prompts` row a session restores
verbatim, and the goals in `state_meta` all ride along) plus
`memories/MEMORY.md` and `memories/USER.md`. Skill writing is not
supported, so nothing else travels. Each chunk leads with a tag
byte — 0, 1, 2 — and **nothing is ever whole in memory**: the
inbound `Ingest` appends each chunk to its file as it lands, keeping
only the open handle; the outbound `stream` folds the database
(`VACUUM`, then `wal_checkpoint(TRUNCATE)`, then the last close —
required, because Hermes's own close checkpoints only passively) and
reads each file in 2 MiB pieces, one alive at once. A delivered
database is proved to open with `quick_check` before the gateway
starts, because Hermes would otherwise heal a corrupt one by
quarantining it and starting fresh — silent amnesia. The fetch
answers the SESSION ID to resume, read from the database as its most
recently active row, since compaction moves the tip. The user
trimmed the module to its flow: the container is fresh and the
delivery lands before the gateway starts, so it clears nothing and
polices no order.

## Hermes: the filesystem, the fetcher, the stores

`filesystem::prepare(&Request, Fetcher)` renders the whole
pre-gateway filesystem in one pass: the gateway's process environment
(43 provider arms, the toolset credentials, `HERMES_HOME`,
`HERMES_WRITE_SAFE_ROOT=` empty, the yolo switch, an
`API_SERVER_KEY` minted per run as 64 hex characters), `config.yaml`
written as JSON (JSON is YAML) with `model.provider` = the marker's
own id, the backend pins, the explicit `platform_toolsets.api_server`
list (Hermes has no `disabled_toolsets` key), the MCP proxy entry
with elicitation and sampling off, `skills.external_dirs`; and the
credential files — `auth.json` entries verbatim, the Qwen CLI file at
the real home, the vertex file. Resources are fetched all at once and
the continuation's settlement is awaited beside them; each file is
written exactly once. Four bugs were caught in review against the
Hermes source: `custom` needs `model.base_url` beside `model.api_key`
or the key is ignored; `image_gen` is off unless keyed; Browserbase
pins `browser.cloud_provider`; and `HERMES_HOME` is pinned rather
than trusted to `HOME`. `filesystem::finish(&Request)` is the way
back up — every named resource as the run left it (a quarantined
entry yields nothing), then the continuation. `filesystem::history`
mirrors Hermes's own loader reduced to what `/v1/runs` can carry
(role + content text; the runs handler loads no history itself, the
chat-completions handler does — verified in the source), and
`filesystem::session` re-reads the tip between turns.

One `Fetcher`, by value, with one ask channel and two methods —
resources and the continuation were the same job in different
shapes; two stores under `store::` with the same verbs, differing
only in where the bytes go.

## Hermes: the run

`run::raw::run` posts `/v1/runs`, subscribes once to its event
stream (no reconnect: one destructive reader) and yields each frame
as the typed `Event`. `run::run` owns the whole lifetime — prepare,
spawn `hermes gateway`, poll `/health` with no timeout, drive turns,
SIGTERM, finish — and converts events into chunks: FIFO-paired tool
calls under random base62 ids of 22 characters (no hex, by request),
the started preview as the call's arguments and an EMPTY tool
response on completion (the proxy reports nothing to us; only its
`/enqueue` returning at the fold is observable), deltas and reasoning
as text, `run.completed`'s output spoken only when no delta came, its
usage as the bill, `run.failed` fatal. The queue is the container's
own, mirrored onto the proxy's: a fate is decided by whoever takes it
first — the fold (`delivered`, yielded after the tool response whose
completion is the first at or after the fold's moment), the caller's
dequeue, or the turn's end, where anything pending is withdrawn at
the proxy and becomes the next turn's prompt on the same session.
`main.rs` is the socket around it: the fetcher's asks and the run's
items merged onto one socket as their frames. Zero warnings.

## Hermes: the image, and skills

The runtime is Hermes's own published image — already Debian —
pinned by tag and digest to the first release after the pinned
source commit, with a build-time assertion that `hermes version` is
0.20.6; `ddgs`, `edge-tts` and `fal-client` installed into its venv
(lazy installs are disabled there); our two musl binaries; our
entrypoint in place of its s6 dispatcher so no gateway auto-starts.
Skills come from mounts at `/root/.hermes/external-skills/`, named
to Hermes as an external skill directory it reads recursively and
never writes (its own `skills/` is synced into and bookkept); the
`skills` toolset is on exactly when something is mounted there.

## Spec pages

`agentic-loop-run/{index, request/index, response}.mdx` follow the
mounts and the environment, the continuation as bytes, the kept
chunks and the resource frame. The rest of the spec is behind — see
below.

## What remains

- **Nothing has been built or run live.** No Containerfile in this
  period was built; no container has served a run. cc, openrouter
  and hermes are all "compiles clean, never executed". Image builds
  and live runs need express approval by the standing rule.
- **The server-side relay** — the provider server that opens the
  container's socket, re-tags frames 2/3/4 onto the wire and consumes
  0/1, serving `fetch_resource` and `fetch_continuation` from the
  client and POSTing deliveries — lives outside this repository.
- **Byte-faithful tool responses for Hermes.** The proxy has no
  socket to the loop container, so proxied MCP results are as
  invisible to hermes as built-in ones; every tool response is empty
  today. A relay inside the hermes binary in front of the proxy would
  give full fidelity for MCP tools.
- **Resource frames from cc and openrouter** — neither has anything
  to emit yet; hermes emits them from `finish`.
- **pi and eliza** are skeletons with no research beyond eliza's
  report 1.
- **Spec debt** — the fetch tags 5–8, client-opened channels, the
  `user` chunk, the queue verbs, the WebSocket container surface, the
  hermes agent and its provider/toolset vocabularies await prose.
- **Hermes unknowns only a run will settle** — whether the API
  server auto-enables `x_search`/`homeassistant` under an explicit
  list, whether a run creates exactly one lineage (the tip query
  assumes it), and whether the pinned image tag truly carries 0.20.6
  (the build asserts it).

Two crate-local trails continue: this series for the protocol, and
`diverge-agentic-loop-hermes/reports/` plus its `HARNESS.md` for the
Hermes detail.
