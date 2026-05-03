# objectiveai-api

## Stream-First Architecture

The API server is built on a **stream-first, zero-collect** philosophy. Every layer produces a `Stream` of typed chunks and yields them immediately to the consumer. Nothing is buffered or collected in the hot path — chunks flow from upstream LLM providers through to the HTTP response with minimal per-chunk overhead.

This enables:
- **Consistent progress notifications** at every level (upstream tokens → agent completions → vector votes → function execution → invention steps)
- **Low memory overhead** — no intermediate `Vec<Chunk>` allocations in the streaming path
- **Composable concurrency** — parallel streams are merged via `select_all`, not joined-then-collected

### Layered Streaming Architecture

The system has 4 layers, each defining its own chunk type and wrapping the layer below:

```
Layer 4: Function Executions / Inventions
         ↓ FunctionExecutionChunk / FunctionInventionChunk
Layer 3: Vector Completions
         ↓ VectorCompletionChunk
Layer 2: Agent Completions
         ↓ AgentCompletionChunk (+ Continuation state)
Layer 1: Upstream Clients (OpenRouter, Claude Agent SDK, Mock)
         ↓ AgentCompletionChunk (+ upstream-specific State)
```

Each layer wraps the inner layer's chunks into its own chunk type and yields immediately — **passthrough, not collect-then-transform**.

### Upstream Clients (Layer 1)

The `UpstreamClient<AGENT>` trait is generic over agent type:

```rust
trait UpstreamClient<AGENT> {
    type State;
    type Stream: Stream<Item = StreamItem<Self::State>>;
    fn create(...) -> Result<(Self::Stream, Self::State), Self::Error>;
}
```

`StreamItem<STATE>` has two variants: `Chunk(AgentCompletionChunk)` for streaming data, and `State(STATE)` as the final item carrying continuation state.

**Contract:** The first stream item must never be an error chunk. If the upstream fails before producing a non-error chunk, `create()` must return `Err(...)`. All three upstreams enforce this by awaiting the first item inside `create()` itself.

- **OpenRouter:** SSE via `reqwest_eventsource`. Each SSE event is converted 1:1 into an `AgentCompletionChunk` via `into_downstream()`. Accumulates an `AssistantMessage` for continuation state (the only collection — required for tool-call loops).
- **Claude Agent SDK:** Spawns a `node` subprocess, reads NDJSON lines from stdout. Each `SDKMessage` is converted to `AgentCompletionChunk`. Unknown message types are silently skipped (forward-compatible).
- **Mock:** Generates random responses from a seeded RNG. Yields pre-computed chunks one by one with optional delay. Usage is always `None`.

### Agent Completions Client (Layer 2)

Orchestrates upstream selection, backoff retry across agents (primary + fallbacks), and the tool-calling loop.

**`run_agent_loop()`** is the core: it creates the upstream stream and enters an `async_stream::stream!` that yields chunks as they arrive. Uses a **one-ahead buffer** — the previous chunk is yielded immediately, the current chunk is held — so usage can be attached to the final chunk without collecting everything.

After each assistant turn, checks for callable tool calls in the accumulated response. If tools were called, executes them (MCP or invention tools), yields tool-response chunks, pushes results onto the continuation, and calls `upstream.create()` again for the next turn. All of this happens within the same stream — no intermediate collection.

### Vector Completions Client (Layer 3)

Fans out to multiple agent completions in parallel via `futures::stream::select_all`.

Each LLM's agent stream is wrapped: every `AgentCompletionChunk` becomes a `VectorCompletionChunk` via **immediate passthrough** (`wrap_agent_chunk()`). After an LLM's stream ends, the vote is extracted from the accumulated response and yielded as a final chunk.

The main stream uses the same **one-ahead buffer** pattern: it updates running `weights` and `scores` from each vote as it arrives and attaches cumulative scores to every outgoing chunk. The consumer sees scores converge in real time as votes arrive.

### Function Executions Client (Layer 4a)

Fetches function + profile, flattens into `FlatTaskProfile` tasks, executes all vector completion tasks via the vector client's `create_streaming()`. Each task's vector stream is wrapped into `FunctionExecutionChunk` — **immediate passthrough**. After all tasks complete, applies output expressions and optionally runs a reasoning summary via the agent client.

### Function Inventions Client (Layer 4b)

Runs 5 sequential steps (essay, input schema, essay tasks, tasks, description). Each step calls `agent_client.create_streaming()` with invention tools. Every `StreamItem::Chunk` from the agent stream is wrapped into a `FunctionInventionChunk` — **immediate passthrough**. Between steps, state chunks are yielded to update the consumer on invention progress. If validation fails, retries by pushing a retry prompt onto the continuation within the same stream.

### The `create_streaming_handle_usage()` Pattern

Every layer (agent, vector, function executions, inventions) follows the same pattern for decoupled usage reporting:

```
let (tx, rx) = unbounded_channel();
tokio::spawn(async move {
    // call create_streaming(), send chunks through tx, aggregate in parallel
    // after stream ends: convert aggregate to unary, call usage_handler
});
// await first item from rx for error-or-first-chunk check
// return StreamOnce(first).chain(rest)
```

This means:
- The consumer sees a normal stream — chunks arrive immediately
- Usage aggregation happens in a background task, in parallel with consumption
- Usage is reported exactly once after the stream ends via a `UsageHandler` trait
- The first-item await ensures leading errors are returned as `Err(...)`, not as stream items

`StreamOnce` (from `util.rs`) re-emits the first chunk that was consumed during the error check, so no chunks are lost.

### One-Ahead Buffering

Both the agent completions client and vector completions client use one-ahead buffering: the previous chunk is yielded while the current chunk is held. This lets the **last** chunk be annotated with accumulated usage or final scores before yielding, without requiring the stream to collect or look ahead.

### Key Design Rules

1. **Never collect chunks in the streaming path.** The only `create_unary_*` methods that collect are explicitly for non-streaming use.
2. **Yield immediately, annotate later.** Cumulative scores, usage, and state are attached to chunks as they pass through — not computed after the fact.
3. **Usage handlers run after the stream ends.** They receive the aggregated unary response and are never in the critical streaming path.
4. **First-chunk contract at every layer.** If the first item would be an error, `create()` returns `Err(...)` instead of yielding an error chunk. This gives callers a clean `Result` to handle.
5. **Passthrough wrapping, not transformation.** Each layer wraps inner chunks into its own type without modifying or buffering the inner data.

## Testing

- **Never use `--test-threads=1`** when running tests. It makes test runs extremely slow. Let tests run in parallel (the default).

### Snapshot Tests

Several test suites use snapshot files. Set the corresponding env var to `1` to regenerate snapshots. **Always run snapshot updates in a single command** — do NOT run them as separate commands (they share a Cargo build lock and will conflict).

| Env Var | Test Filter | Snapshot Directory |
|---------|-------------|--------------------|
| `UPDATE_AGENT_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS` | `agent::completions::client_tests` | `assets/agent/completions/client_tests/` |
| `UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS` | `vector::completions::client_tests` | `assets/vector/completions/client_tests/` |
| `UPDATE_FUNCTIONS_EXECUTIONS_CLIENT_TESTS_SNAPSHOTS` | `functions::executions::client_tests` | `assets/functions/executions/client_tests/` |
| `UPDATE_FUNCTIONS_INVENTIONS_CLIENT_TESTS_SNAPSHOTS` | `functions::inventions::client_tests` | `assets/functions/inventions/client_tests/` |
| `UPDATE_FUNCTIONS_INVENTIONS_RECURSIVE_CLIENT_TESTS_SNAPSHOTS` | `functions::inventions::recursive::client_tests` | `assets/functions/inventions/recursive/client_tests/` |
| `UPDATE_LABORATORIES_EXECUTIONS_LOCAL_CLIENT_TESTS_SNAPSHOTS` | `laboratories_executions` | `assets/laboratories/executions/local/client_tests/` |

Example — regenerate all invention snapshots (regular + recursive) in one command:

```bash
UPDATE_FUNCTIONS_INVENTIONS_CLIENT_TESTS_SNAPSHOTS=1 \
UPDATE_FUNCTIONS_INVENTIONS_RECURSIVE_CLIENT_TESTS_SNAPSHOTS=1 \
cargo test -p objectiveai-api "functions::inventions"
```

Example — regenerate all snapshots at once:

```bash
UPDATE_AGENT_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS=1 \
UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS=1 \
UPDATE_FUNCTIONS_EXECUTIONS_CLIENT_TESTS_SNAPSHOTS=1 \
UPDATE_FUNCTIONS_INVENTIONS_CLIENT_TESTS_SNAPSHOTS=1 \
UPDATE_FUNCTIONS_INVENTIONS_RECURSIVE_CLIENT_TESTS_SNAPSHOTS=1 \
UPDATE_LABORATORIES_EXECUTIONS_LOCAL_CLIENT_TESTS_SNAPSHOTS=1 \
cargo test -p objectiveai-api
```
