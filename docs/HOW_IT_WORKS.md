# How ObjectiveAI works

> The deep dive. If you just want to install and run something, start with the
> [README](../README.md). This document explains the machinery underneath:
> how a swarm votes, how votes combine, and how the pieces fit together.
> Everything here describes the open-source code in this repository (MIT).

ObjectiveAI is **a harness for swarms of LLM agents**. You define and compose
swarms and run them three ways:

- **Agent completion** — spawn a single agent to *do work* (tools, MCP, multi-turn).
- **Vector completion** — spawn a swarm to *score* something → a vector that
  sums to 1 (or a scalar in [0,1]). This is the swarm-scoring primitive the
  platform is named for.
- **Swarm sandbox** — hand a swarm a Docker sandbox to operate in.

Everything is **content-addressed Git JSON**, executed by a Rust monorepo with
SDKs in five languages, a CLI, a desktop viewer, a web app, and an MCP server.

## The idea

A single sampled token discards everything the model "almost said."
ObjectiveAI also deliberately avoids asking models to self-report confidence —
LLMs are not good at self-assessing. Instead, confidence is **measured** from
the token distribution and **combined** across models:

1. Read each model's logprobs to recover its full preference distribution over
   the candidate answers — not just its top token.
2. Combine those distributions across a swarm of models under learned weights.
3. The result is a score vector that sums to 1: peaked = agreement, flat = no
   signal.

## Probabilistic voting

Source: `objectiveai-api/src/vector/completions/{pfx.rs, get_vote.rs, response_key.rs}`

- Candidate responses are labeled with backtick keys `` `A` ``…`` `T` `` —
  exactly 20 leaves, matching the typical `top_logprobs=20` width.
- The model is forced to emit a key via its `output_mode`
  (`instruction` | `json_schema` | `tool_call`).
- The engine **does not trust the sampled key**. It walks the `top_logprobs`
  at the key's token position, computes `probability = logprob.exp()` for each
  alternative, accumulates per candidate, and L1-normalizes — recovering a
  real distribution over the candidates from a single completion.
- **More than 20 options:** the prefix tree nests (`` `A``A` ``, `` `A``B` ``)
  so each branch fits one logprobs batch. The first-level prefix is a hard
  route — the model's first token deterministically selects a ≤20-leaf
  sub-branch; the second-level logprobs then give a soft distribution within
  that branch.
- Keys and tree order are RNG-shuffled **per agent**, seeded deterministically
  from the request `seed` plus the agent's position and the content-addressed
  prompt/response ids — so shuffles are reproducible and the request `seed` is
  the only knob.
- Fallbacks: if a model exposes no logprobs, its vote is one-hot on the
  sampled key; if no key matches at all, the vote is uniform.

## Consensus aggregation

Source: `objectiveai-api/src/vector/completions/client.rs`

```rust
let mut weights = vec![Decimal::ZERO; R];
let mut scores  = vec![Decimal::ONE / Decimal::from(R); R];   // uniform prior
// ... per agent chunk, streaming:
for vote in &chunk.votes {
    for (i, v) in vote.vote.iter().enumerate() {
        weights[i] += *v * vote.weight;                       // accumulate
    }
}
let weight_sum: Decimal = weights.iter().sum();
if weight_sum > Decimal::ZERO {
    for (i, score) in scores.iter_mut().enumerate() {
        *score = weights[i] / weight_sum;                     // L1-normalize → sums to 1
    }
}
```

**`scores[i] = Σₐ(wₐ · voteₐ[i]) / Σ weights`** — a profile-weighted average
of every agent's preference distribution, L1-normalized. Discrete votes become
a weighted plurality; probabilistic votes become a weighted mixture. Scores
update **online** as each agent reports, so streaming clients watch consensus
form in real time.

A per-agent `invert` flag maps each vote component `v → (1 − v)` before
renormalizing — inverting the vote distribution, not the weight.

**Precision:** all scores and weights are `rust_decimal::Decimal`, never
`f64`. Combined with content-addressing and commit-pinning, scoring is
**exactly reproducible**.

## The resources

All resources are JSON files in a Git repo, referenced by
`(owner, repository, commit)` triples and content-addressed via
XXHash3-128 → a 22-character base62 id (computed after normalization, so
semantically identical configs dedupe).

| Resource | File | What it is |
|---|---|---|
| **Agent** | `agent.json` | One fully-specified model config. `upstream ∈ {openrouter, claude_agent_sdk, codex_sdk, mock}`. |
| **Swarm** | `swarm.json` | Ordered, deduped set of `(agent, count)` slots; total count 1–128. |
| **Profile** | `profile.json` | The **learned weights** over a Function's task tree — per-task, or `Auto` (one swarm + uniform weights for every scoring task). |
| **Function** | `function.json` | A recursive evaluation pipeline → one score in [0,1] (`scalar.function`) or a vector summing to 1 (`vector.function`). |

**Resource graph:** `agent ← swarm ← profile`, and separately `function`; at
execution time, **function + profile + input → scores**. The same Function
runs with different Profiles. Profile↔function task alignment is strictly
positional (index-by-index); a length mismatch is an eager, hard
`InvalidProfile` error (HTTP 400) — never a truncate, zero-pad, or silent
misweight.

**Function tasks** (`TaskExpression`): `vector.completion` (a swarm-vote
leaf — messages + candidate responses), `scalar.function` / `vector.function`
(recurse into nested functions), and placeholder variants for scaffolding.
`skip` (boolean expression) drops a task; `map` expands a task once per input
row. An "alpha" authoring DSL transpiles branch/leaf shorthand into the
standard form — the branch/leaf split *is* the recursive scoring tree.

## The expression system

Source: `objectiveai-sdk-rs/src/functions/expression/`

Each task carries an `output` expression (plus optional `input` / `skip` /
`map`). Three expression kinds, tagged in JSON:

- `{"$jmespath": "..."}` — JMESPath with custom functions (`add, subtract,
  multiply, divide, if, zip_map, l1_normalize`, …). `l1_normalize` is the
  canonical "make it sum to 1" op.
- `{"$starlark": "..."}` — Starlark (Python-like, not Turing-complete), used
  to build message/response arrays programmatically.
- `{"$special": ...}` — built-ins: `Input`, `Output`,
  `TaskOutputL1Normalized`, `TaskOutputWeightedSum`, and friends.

`TaskOutputWeightedSum` is the calibration trick that turns a vote vector into
a scalar: response *i* of *N* sits at `i/(N−1)` on a 0→1 ruler, so
`[Yes, No]` yields `p(No)` and a 5-point scale becomes
`0·s₁ + 0.25·s₂ + 0.5·s₃ + 0.75·s₄ + 1·s₅`.

## Running things

### Vector completion

`POST /vector/completions` with `{ messages, responses (≥2), swarm, seed?,
stream?, … }` — weights live inside `swarm.weights`. Returns
`{ scores (sums to 1), weights (raw per-response totals), votes[],
completions[], usage }`, streamable over SSE or WebSocket.

### Function execution

`POST /functions/executions` with `{ function, profile, input, strategy?,
seed?, … }`. The engine resolves and transpiles the function and profile,
aligns each task to its profile entry, compiles tasks against the input
(evaluating `skip`/`map`/`input`), then per task: a `vector.completion` runs a
swarm vote, nested functions recurse, placeholders return constants. Each
task's `output` expression is applied, and the function result is the
profile-weighted, L1-normalized average over tasks.

### Swiss tournament

For ranking many items without O(N²) comparisons:
`Strategy::SwissSystem { pool = 10, rounds = 3 }` splits the input into pools,
runs all pools concurrently each round, re-sorts by cumulative score between
rounds (Swiss pairing), and reports the per-item mean over rounds,
L1-normalized.

### Agent completions

The agent loop is upstream-agnostic: stream chunks from the upstream, extract
callable MCP tool calls, dispatch them over a per-agent MCP proxy, feed
results back as a continuation, repeat until no tool calls remain. For the
Claude Agent SDK and Codex SDK upstreams, long-lived runner subprocesses
multiplex many concurrent sessions over NDJSON. Client-hosted MCP servers and
resources are reached back over the request's own WebSocket (the
reverse-channel), so an agent running in the cloud can call tools that live
on your machine.

## Profiles & the tier ladder

Five official profiles are published under
[github.com/ObjectiveAI](https://github.com/ObjectiveAI) as `profile-*` repos:

| | nano | mini | standard *(default)* | giga | giga-max |
|---|---|---|---|---|---|
| LLM entries | 4 | 4 | 9 | 12 | 7 |
| samples/eval (Σ count) | 4 | 12 | 17 | 20 | 13 |
| philosophy | fast horizontal | 3× nano | reasoning + base blend | frontier-anchored | pure frontier |

nano→mini is pure horizontal sampling (same models, count 1→3); standard adds
reasoning models at full weight and demotes the base models; giga-max
triple-samples three frontier models with a handful of cheap models at weight
0.01 purely for tie-breaking.

## Profile training

**ObjectiveAI does not fine-tune models. It learns weights.** Models stay
fixed; only the combination weights — over agents within a swarm, and over
tasks within a function — are fit to data.

`POST /functions/profiles/compute` takes
`{ function, n, dataset: [{input, target}], swarm, seed, max_retries }` and
returns fitting stats. The production weight-optimization service runs hosted
(the open-source API server forwards these requests); the optimizer itself is
proprietary. The
[benchmarks repo](https://github.com/ObjectiveAI/llm-weighted-consensus-benchmarks)
contains an offline, readable analogue used for the published experiments.

## Architecture

`ObjectiveAI/objectiveai` is a polyglot Cargo + pnpm workspace with a single
shared version. The core is Rust.

| Package | Role |
|---|---|
| `objectiveai-sdk-rs` (+ macros/cffi/pyo3/wasm) | SDK core / source of truth: types, validation, content-addressing, expression engine |
| `objectiveai-api` | axum API server + execution engine; self-host runs as a local sidecar that executes locally and forwards auth + profile-training to the hosted API |
| `objectiveai-cli` | the `objectiveai` CLI (NDJSON stdout) |
| `objectiveai-mcp` (+ proxy/filesystem) | MCP servers; the reverse-channel lives here and in `objectiveai-cli/src/websockets/` |
| `objectiveai-sdk-{js,py,go,dotnet}` | generated SDKs — identical field names in every language |
| `objectiveai-json-schema` | ~1,000 generated schema files: the polyglot type contract |
| `objectiveai-viewer` / `objectiveai-web` | Tauri desktop viewer / Next.js web app |

The SDK pipeline is codegen: Rust types derive JSON Schema, a builder
normalizes and emits the schema files, and per-language installers generate
Zod/TypeScript, Pydantic/Python (with a PyO3 native extension), Go (wazero-
hosted WASM core, no CGO), and .NET bindings. Roundtrip tests assert that
re-serialization equals the source schema.

The MCP server exposes one static `ObjectiveAI` tool that proxies the entire
CLI (`{ command: [argv], timeout, jq?, python? }` with per-line post-
processing), plus dynamic per-plugin and per-tool routes.

## Plugins

Plugins extend agents with daemons, MCP servers, and hooks:

- [`psychological-operations`](https://github.com/ObjectiveAI/psychological-operations)
  — the flagship: an ingest → filter → sort → dedup → score → deliver pipeline
  for X and Discord, where the scoring stages are ObjectiveAI Functions +
  Profiles, feeding persona agents that act through 20+ MCP tools.
- [`mundus-animarum`](https://github.com/ObjectiveAI/mundus-animarum) — "world
  of souls": mutable per-agent identity/memory KV keyed to the agent's
  immutable hash id, with cross-agent subscriptions and notifications.
- [`quas-wex-exort`](https://github.com/ObjectiveAI/quas-wex-exort) —
  programmatic MCP-tool/CLI invocation for agents.
- [`arcanum`](https://github.com/ObjectiveAI/arcanum) — skills loading and
  per-agent governance (early).

## Benchmarks

[`llm-weighted-consensus-benchmarks`](https://github.com/ObjectiveAI/llm-weighted-consensus-benchmarks)
— reproducible, raw artifacts committed, honest about negative results:

- **Humanity's Last Exam:** a heterogeneous swarm (`gpt-5-mini ×3 +
  gemini-2.5-flash ×3`) scored **20.83%**, beating either model alone
  (best single: 18.88%). One tested swarm *lowered* accuracy — reported as a
  counterexample.
- **IT ticket classification:** a single LLM read via `top_logprobs=20`
  scored **45.13%** vs 39–42% for chat-style prompting, at roughly a tenth of
  the cost; learned weights pushed winner-accuracy to 47.1%.
- **Poll approximation:** simulation did **not** beat single-LLM
  approximation — a reported null result.

The supported thesis: a weighted ensemble of cheaper models beats one
expensive model on bounded-choice tasks, at lower cost. Not a
general-intelligence claim.

## Glossary

The concepts have carried three vocabularies over the project's history:

| Concept | v1 gRPC era | sandbox era | current |
|---|---|---|---|
| swarm of models | "Query Model" | `ensemble` | `swarm` / `agents` |
| weights | per-LLM `weight` | `profile` (number[]) | `weights` (+ `invert`) |
| scoring call | `/score/completions` | — | `/vector/completions` |

**Function** = what to ask · **Profile** = how votes combine · **Swarm** =
which models · **Agent** = one model's config. The word "judgment" survives as
the name of the *mechanism* — routing a decision through a voting swarm.

## Source map

Where to read the load-bearing code:

- Logprob → distribution: `objectiveai-api/src/vector/completions/get_vote.rs`
- Prefix tree: `objectiveai-api/src/vector/completions/pfx.rs`
- Consensus aggregation: `objectiveai-api/src/vector/completions/client.rs`
- Swarm hashing: `objectiveai-sdk-rs/src/swarm/swarm.rs`
- Function/Task types: `objectiveai-sdk-rs/src/functions/{function.rs, task.rs}`
- Expression runtime: `objectiveai-sdk-rs/src/functions/expression/`
- Function combine + Swiss system: `objectiveai-api/src/functions/executions/client.rs`
- Profile↔function alignment: `objectiveai-api/src/functions/flat_task_profile.rs`
- API routes: `objectiveai-api/src/run.rs`
- Agent loop: `objectiveai-api/src/agent/completions/client.rs`
- Reverse-channel: `objectiveai-mcp-proxy/src/reverse_channel.rs`, `objectiveai-cli/src/websockets/conduit.rs`
