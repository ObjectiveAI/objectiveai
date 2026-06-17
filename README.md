# ObjectiveAI

**The Swarm Judgment Harness.**

Define and compose swarms of LLM agents. Spawn an agent to do things, spawn a swarm to evaluate things, or hand a swarm a Docker sandbox — from the CLI, the SDKs, or your own agent.

[Website](https://objectiveai.dev) · [Discord](https://discord.gg/gbNFHensby) · [GitHub](https://github.com/ObjectiveAI/objectiveai)

[![Release](https://img.shields.io/github/v/release/ObjectiveAI/objectiveai?label=release&color=blue)](https://github.com/ObjectiveAI/objectiveai/releases/latest)
[![Crates.io](https://img.shields.io/crates/v/objectiveai-sdk?label=crates.io%20%2F%20objectiveai-sdk)](https://crates.io/crates/objectiveai-sdk)
[![npm](https://img.shields.io/npm/v/@objectiveai/sdk?label=npm%20%2F%20%40objectiveai%2Fsdk)](https://www.npmjs.com/package/@objectiveai/sdk)
[![PyPI](https://img.shields.io/pypi/v/objectiveai-sdk?label=pypi%20%2F%20objectiveai-sdk)](https://pypi.org/project/objectiveai-sdk/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Packages

SDKs published to language-native registries. Pick the one for your stack:

| Language | Package | Install |
|---|---|---|
| Rust | [`objectiveai-sdk`](https://crates.io/crates/objectiveai-sdk) | `cargo add objectiveai-sdk` |
| TypeScript | [`@objectiveai/sdk`](https://www.npmjs.com/package/@objectiveai/sdk) | `npm i @objectiveai/sdk` |
| Python | [`objectiveai-sdk`](https://pypi.org/project/objectiveai-sdk/) | `pip install objectiveai-sdk` |
| Go | [`objectiveai-sdk-go`](https://pkg.go.dev/github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go) | `go get github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go` |

Additional crates on crates.io: [`objectiveai-api`](https://crates.io/crates/objectiveai-api), [`objectiveai-cli`](https://crates.io/crates/objectiveai-cli), [`objectiveai-mcp`](https://crates.io/crates/objectiveai-mcp), [`objectiveai-mcp-proxy`](https://crates.io/crates/objectiveai-mcp-proxy), [`objectiveai-mcp-filesystem`](https://crates.io/crates/objectiveai-mcp-filesystem), [`objectiveai-sdk-macros`](https://crates.io/crates/objectiveai-sdk-macros). Additional PyPI package: [`objectiveai-cocoindex`](https://pypi.org/project/objectiveai-cocoindex/).

## Binaries

Install all four prebuilt binaries with one command:

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
. "$HOME/.objectiveai/env"
```

| Binary | What it does | Download |
|---|---|---|
| `objectiveai` | CLI + embedded viewer | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |
| `objectiveai-api` | API server | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |
| `objectiveai-viewer` | Standalone Tauri desktop app | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |
| `objectiveai-mcp` | MCP server (streamable HTTP) | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |

Supported platforms: Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64, Windows x86_64. See [Binaries & self-hosting](#binaries--self-hosting) for install flags and per-binary detail.

---

## What ObjectiveAI is

ObjectiveAI is a harness for defining, composing, and running swarms of LLM agents. You define an **Agent** once — model, prompts, decoding parameters, output mode, tools, MCP servers. You compose Agents into a **Swarm**. You then run them in any of three execution modes: spawn a single agent to do work, spawn a whole swarm to collectively evaluate something, or hand a swarm a Docker sandbox to act in.

Agents, Swarms, Functions, and Profiles are all content-addressed Git-hosted resources. The same swarm.json that powers your CLI invocation tonight is the one your colleague pins by commit SHA next month and the one your trained Profile was fit against.

The brand promise is **judgment**: the system was built so that collective evaluation by a swarm — not a single sampled token — produces every score. The mechanism is **swarms**: reusable, composable, version-tracked collections of configured models. Everything else (the CLI, the API, the web app, the MCP server, the SDKs in five languages) exists to drive swarms in the ways that matter.

### Execution modes

Two shapes. Each mode resolves the same Agents and Swarms but does something different with them.

| Mode | What it does | Returns | Reach for it when |
|---|---|---|---|
| [**Agent completion**](#agent-completions) | Spawn a single Agent to do work — call tools, talk to MCP servers, execute multi-turn loops, generate artifacts | Whatever the Agent produces | You need one agent to perform a discrete task |
| [**Function execution**](#function-executions) | Spawn a swarm to evaluate something. Functions are composable, recursive evaluation pipelines | Scalar or vector of scores | You want a calibrated, trainable multi-model evaluation |

Function execution is the judgment mode — that's where the system's name comes from. Agent completions are the foundational orchestration layer; every other mode is built on top of them.

### Why collective judgment

A single language model asked to score something hands back one sampled token and walks away from everything else it computed. The signal it had — how confident it really was, where it hedged, what it nearly chose instead — never leaves the model. ObjectiveAI is built to preserve that signal across an entire swarm.

Each agent in a swarm contributes a preference distribution over the candidates rather than a single sampled token. Those distributions combine across the swarm under learned weights to produce the final score. No discrete collapse. No lost signal.

```
Function execution requested
        │
        ▼
  ┌─────────────┐
  │  Function   │  (composable, content-addressed, versioned)
  └──────┬──────┘
         │ fans out to its swarm
         ▼
  ┌──────────────────────────────────┐
  │              Swarm               │
  │  ┌────────┐ ┌────────┐ ┌──────┐ │
  │  │ Agent  │ │ Agent  │ │ ...  │ │
  │  └───┬────┘ └───┬────┘ └──┬───┘ │
  └──────┼──────────┼─────────┼─────┘
         │ evaluations         │
         ▼          ▼         ▼
  ┌────────────────────────────────┐
  │  weighted combination (Profile)│
  └────────────────────────────────┘
         │
         ▼
  scores: [0.61, 0.28, 0.11]  (sums to 1)
```

This matters twice over: once per model, and once across models. Different models have different failure modes, different training distributions, different calibration profiles. Combining them with learned weights — weights that can be trained against ground truth — is strictly more powerful than picking the one model that scores highest on average.

### Why this system

Reusability across modes. Trainability where it counts. Content-addressing throughout:

- **Reusable.** An Agent is a 22-character ID — define one once and reference it from any swarm, any function, any lab. A Swarm is a sorted set of `(agent_id, count)` pairs. Define it once and run it for action, evaluation, or sandboxed work without re-defining anything.
- **Trainable.** Profiles learn weights over a Function's task tree against labeled data. The models stay fixed; the way the swarm's evaluations combine improves.
- **Reproducible.** Every resource reference is `(owner, repo, commit)`. Pin a commit SHA, get the exact same agent / swarm / function / profile your evaluation ran against six months ago.
- **Composable.** Functions can call other Functions. Swarms compose into bigger swarms. The CLI dispatches plugins as unknown subcommands. The viewer surfaces plugin UIs as sandboxed iframe tabs.
- **Polyglot.** Rust, TypeScript, Python, Go, and (in-progress) .NET SDKs share the same generated JSON Schema corpus. Field names and shapes are identical across languages.

## Quick start

Install the CLI, API server, viewer, and MCP server from the latest release:

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
. "$HOME/.objectiveai/env"
```

Set your API key:

```bash
objectiveai api headers x-objectiveai-authorization config set "apk_your_key_here"
```

### CLI — spawn a swarm to evaluate something

A **Function** describes the evaluation; a **Profile** names the swarm and its trained weights. Both are GitHub-hosted resources you reference by `owner/repository`. The example below calls a published code-safety evaluator function with its trained profile:

```bash
objectiveai functions executions create standard \
  --function remote=github,owner=your-org,repository=safety-evaluator \
  --profile remote=github,owner=your-org,repository=safety-evaluator-profile \
  --input-inline '{"snippet":"eval(user_input)"}'
```

The streamed output ends with a notification containing the score vector:

```json
{"Notification":{"value":{"execution":{"output":{"Vector":[0.91,0.07,0.01,0.01]}}}}}
```

Each number is the swarm's combined evaluation weight for that label, in the order declared by the function's response set. Values sum to 1.

### SDK — TypeScript

```typescript
import { ObjectiveAI, functionsExecutionsCreateFunctionExecution } from "@objectiveai/sdk";

const client = new ObjectiveAI({ authorization: process.env.OBJECTIVEAI_AUTHORIZATION });

const result = await functionsExecutionsCreateFunctionExecution(client, {
  function: { remote: "github", owner: "your-org", repository: "safety-evaluator" },
  profile: { remote: "github", owner: "your-org", repository: "safety-evaluator-profile" },
  input: { snippet: "eval(user_input)" },
  stream: false,
});

console.log(result.output); // { Vector: [0.91, 0.07, 0.01, 0.01] }
```

### Other execution modes

Spawn a single agent to do work:

```bash
objectiveai agents spawn \
  --agent remote=github,owner=your-org,repository=writer-agent \
  --inline '[{"role":"user","content":"Write a haiku about ocean waves."}]'
```

Pin a `commit=<sha>` segment to lock in a specific version of any remote resource. See [Core primitives](#core-primitives) for a full explanation of Agents, Swarms, Profiles, and the two execution modes, and [SDKs](#sdks) for Python, Rust, Go, and .NET patterns including streaming.

## Core primitives

Three **resources** (Agents, Swarms, Profiles) define what's in the system; two **execution modes** (Agent completions, Function executions) define what you can do with them. Resources are content-addressed Git-hosted JSON; execution modes resolve resources at request time and stream typed results back. Everything ties together through a shared resource graph at the bottom of this section.

### Agents

An **Agent** is a fully-specified configuration of a single upstream model: model identity, prompt structure, decoding parameters, output mode, tools, MCP servers, provider preferences. Agents are content-addressed via XXHash3-128 — the same configuration always produces the same 22-character base62 ID. IDs are deterministic because the serialized configuration is hashed after normalization (empty fields stripped, defaults canonicalized). Two Agents with identical effective settings are the same Agent.

Agents are stored as `agent.json` in Git repositories and referenced by `owner/repo@commit` everywhere a swarm or function needs an agent. Authoring agents lives in source control; calling them happens by reference.

```json
{
  "description": "Skeptical evaluator",
  "upstream": "openrouter",
  "model": "openai/gpt-4o",
  "output_mode": "json_schema",
  "temperature": 0.2,
  "prefix_messages": [
    { "role": "system", "content": "You are a rigorous critic. Challenge assumptions." }
  ]
}
```

Each upstream (OpenRouter, Claude Agent SDK, Codex SDK) has its own agent type with its own parameter set. The same Agent can be driven in either execution mode — running solo in an agent completion, or contributing to the swarm's evaluation in a function execution.

### Swarms

A **Swarm** is an ordered collection of Agents used together for collective judgment. Swarms are immutable and content-addressed — their ID is computed from the sorted `(full_id, count)` pairs of their constituent agents. Weights are **not** baked into the swarm definition; they are execution-time parameters supplied by a Profile or passed directly.

Each agent slot has a `count` (number of instances) and optional fallbacks. Duplicate agents are merged and their counts summed. The total agent count across all slots must be between 1 and 128.

```json
{
  "description": "Balanced judgment panel",
  "agents": [
    {
      "upstream": "openrouter",
      "model": "openai/gpt-4o",
      "output_mode": "json_schema",
      "prefix_messages": [
        { "role": "system", "content": "You are a rational skeptic. Ground every choice in logic." }
      ],
      "count": 2
    },
    {
      "upstream": "openrouter",
      "model": "anthropic/claude-sonnet-4-20250514",
      "output_mode": "tool_call",
      "suffix_messages": [
        { "role": "system", "content": "You are an intuitive thinker. Trust your instincts." }
      ],
      "count": 1
    }
  ]
}
```

Swarms are stored as `swarm.json` in Git repositories and shared across functions. Because weights are external, the same swarm can be reused with different weight configurations without creating a new swarm.

### Profiles

ObjectiveAI does not fine-tune models. It learns weights.

A Profile is the result of training: given a dataset of `(input, expected_output)` pairs, ObjectiveAI executes a Function repeatedly, computes loss against expected outputs, and adjusts the weights over each task until they converge. The learned configuration — which tasks to trust more, which to discount — is stored as `profile.json`.

Profiles are GitHub-hosted and referenced by `owner/repo@commit`. Pinning a commit SHA is strongly recommended: the Profile's shape (number of tasks, their order) is tied to the function it was trained on, and that function may evolve. A mismatched Profile silently produces wrong weights.

```json
{
  "owner": "ObjectiveAI",
  "repo": "quality-scorer",
  "commit": "a3f8c21d..."
}
```

At execution time, the Function and Profile are independent inputs. The retrieval system fetches both, resolves the resource graph, and applies the learned weights to combine task outputs.

### Agent completions

An **agent completion** spawns a single Agent to do work. The Agent receives a task as a conversation and acts on it — calls tools, talks to MCP servers, executes a multi-turn loop, writes code, generates artifacts. Function executions are built on top of agent completions; they're multi-agent orchestrations of the same underlying primitive.

The Agent is supplied by remote reference. Messages can include images, audio, and files in addition to text. Tool calls are detected mid-stream and executed automatically; MCP servers attached to the Agent are dialed transparently. The response carries a `Continuation` that captures the conversation state so the next call can pick up where this one left off.

```json
{
  "agent": { "remote": "github", "owner": "your-org", "repository": "writer-agent" },
  "messages": [
    { "role": "user", "content": "Rewrite this commit message as a conventional-commits changelog entry." }
  ]
}
```

CLI: `objectiveai agents spawn --agent remote=github,owner=...,repository=... --inline '...'`. SDK: `agentsCompletionsCreateAgentCompletion` (JS) / `create_agent_completion` (Python) / `agent::completions::http::create_agent_completion` (Rust).

### Function executions

A **function execution** spawns a swarm to evaluate something. Functions are composable, recursive evaluation pipelines: data in, scores out. A Function is a list of **tasks** executed against an input. Each task is one of:

- A **swarm evaluation step** — hands the input plus a fixed set of candidate labels to the swarm and gets back a score across those candidates. Each Agent contributes a preference distribution over the candidates rather than a single sampled token; the distributions combine with the Profile's weights to produce the step's score. Larger candidate sets are handled transparently by internal machinery — evaluations span hundreds of candidates if needed.
- A **nested function call** — references another `function.json` by `owner/repo@commit`. Resolves and executes that function against the same (or a transformed) input.
- A **mapped operation** — runs a task N times over an indexed range, producing N outputs.

Functions are recursive: a function's tasks can themselves be functions, which can contain more nested functions. The composition is arbitrarily deep.

Functions produce either:

- **Scalar** — a single score in [0, 1].
- **Vector** — an array of scores summing to 1, one per output dimension.

The final output is the weighted combination of all task outputs, with weights supplied by a Profile. Tasks carry `output` expressions (JMESPath or Starlark) that transform raw task results into the function's output type before combining.

Functions are stored as `function.json` in Git repositories and referenced by `owner/repo` triple. They are content-addressed via their task structure and input schema.

```json
{
  "type": "alpha.scalar.leaf.function",
  "description": "Score response quality on a 0-1 scale",
  "input_schema": { "type": "object", "properties": { "response": { "type": "string" } } },
  "tasks": [
    {
      "type": "vector.completion",
      "messages": [{ "role": "user", "content": "Rate this response: {{input.response}}" }],
      "responses": ["poor", "mediocre", "good", "excellent"],
      "output": { "$starlark": "output['scores'][2] + output['scores'][3]" }
    }
  ]
}
```

### The resource graph

All resources reference each other via `(owner, repository, commit)` triples. Content-addressing plus commit pinning makes the full graph reproducible from any entry point.

```text
agent.json  <-  swarm.json  <-  profile.json      function.json
                 (agents)        (swarms+weights)   (tasks + input_schema)

At execution:  function.json + profile.json  ->  scores
```

The Function and Profile are deliberately separate files. The same Function can be run with different Profiles (e.g. a domain-specific profile vs. a general-purpose profile). The same Profile cannot be applied to a structurally different Function — the task count and order must match.

Remote references resolve lazily: the retrieval system walks the graph starting from the execution request, fetching and caching each resource exactly once. Deduplication is by `(owner, repo, commit)` triple. All fetches are content-verified — a cached resource is never re-fetched if the commit SHA matches. Even deeply nested function graphs execute with minimal network overhead.

## SDKs

Every SDK exposes the same two execution modes: **Agent Completions** (spawn a single Agent to do work — tools, MCP, multi-turn loops) and **Function Executions** (spawn a swarm to evaluate something — composable evaluation pipelines). Both support streaming via Server-Sent Events. The API emits incremental chunks; each SDK merges them into an accumulating object using an immutable merge system (TypeScript), a mutable push system (Python, Rust, Go), or equivalent. Types are generated from a shared JSON Schema corpus derived from the Rust SDK, so field names and shapes are identical across languages.

### Languages

| Language | Package | Install | Runtime targets |
|---|---|---|---|
| Rust | `objectiveai-sdk` on crates.io | `cargo add objectiveai-sdk` | Any (async via `reqwest` + `tokio`) |
| TypeScript | `@objectiveai/sdk` on npm | `npm i @objectiveai/sdk` | Node.js, Deno, browser (CJS + ESM) |
| Python | `objectiveai-sdk` on PyPI | `pip install objectiveai-sdk` | CPython 3.10+ (includes PyO3 extension) |
| Go | `github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go` | `go get github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go` | Go 1.26+ |
| .NET | `ObjectiveAI` (NuGet — in progress) | not yet published | net10.0 |

### Streaming examples

The base URL defaults to `https://api.objectiveai.dev` in all SDKs. Auth is passed as `OBJECTIVEAI_AUTHORIZATION` (env var) or via the client constructor.

#### TypeScript

```typescript
import {
  ObjectiveAI,
  functionsExecutionsCreateFunctionExecution,
  functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
} from "@objectiveai/sdk";

const client = new ObjectiveAI({ authorization: process.env.OBJECTIVEAI_AUTHORIZATION });

const stream = await functionsExecutionsCreateFunctionExecution(client, {
  stream: true,
  function: { remote: "github", owner: "your-org", repository: "safety-evaluator" },
  profile: { remote: "github", owner: "your-org", repository: "safety-evaluator-profile" },
  input: { snippet: "eval(user_input)" },
});

let acc: any = null;
for await (const chunk of stream) {
  acc = acc ? functionsExecutionsResponseStreamingFunctionExecutionChunkMerged(acc, chunk)[0] : chunk;
}
console.log("output:", acc?.output);
```

#### Python

```python
import asyncio, os
from objectiveai_sdk.client import ObjectiveAI
from objectiveai_sdk.functions.executions.http import create_function_execution

async def main() -> None:
    client = ObjectiveAI(authorization=os.environ.get("OBJECTIVEAI_AUTHORIZATION"))
    params = {
        "stream": True,
        "function": {"remote": "github", "owner": "your-org", "repository": "safety-evaluator"},
        "profile": {"remote": "github", "owner": "your-org", "repository": "safety-evaluator-profile"},
        "input": {"snippet": "eval(user_input)"},
    }
    stream = await create_function_execution(client, params)
    acc = None
    async for chunk in stream:
        if acc is None:
            acc = chunk
        else:
            acc.push(chunk)
    print("output:", acc.output if acc else None)

asyncio.run(main())
```

#### Rust

```rust
use futures::StreamExt;
use objectiveai_sdk::{HttpClient, functions::executions};

#[tokio::main]
async fn main() -> Result<(), objectiveai_sdk::HttpError> {
    let client = HttpClient::builder()
        .authorization(std::env::var("OBJECTIVEAI_AUTHORIZATION").ok())
        .build();

    let mut stream = executions::http::create_function_execution_streaming(
        &client,
        executions::request::params(/* function: remote ref, profile: remote ref, input */),
    ).await?;

    let mut acc: Option<executions::response::streaming::FunctionExecutionChunk> = None;
    while let Some(Ok(chunk)) = stream.next().await {
        match &mut acc {
            Some(a) => a.push(&chunk),
            None => acc = Some(chunk),
        }
    }
    println!("output: {:?}", acc.map(|a| a.output));
    Ok(())
}
```

### Go and .NET

The Go SDK is fully auto-generated from the JSON Schema corpus. Types are strict-validated on unmarshal. The client exposes generic helpers `PostUnary[T]` / `PostStreaming[T]` / `GetUnary[T]` / `DeleteUnary[T]`; endpoint functions such as `FunctionsExecutionsCreateFunctionExecutionStreaming` wrap these. A wazero-hosted WASM binary (compiled from the Rust core) provides chunk-to-unary conversion and merge verification without CGO.

The .NET SDK (`ObjectiveAI`, targeting net10.0) is in active development. The NuGet publish workflow is not yet wired up, so it must be built from source for now.

## Binaries & self-hosting

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
. "$HOME/.objectiveai/env"
```

All four binaries land in `~/.objectiveai/` and are added to `PATH`. The CLI (`objectiveai`) self-updates on startup; re-run the installer to upgrade `objectiveai-api`, `objectiveai-viewer`, and `objectiveai-mcp`.

### `objectiveai` (CLI)

The primary user-facing binary. Built with `clap` derive macros and emits newline-delimited JSON (NDJSON) on stdout. Top-level command groups: `agents`, `swarms`, `functions`, `vector`, `plugins`, `logs`, `instructions`, `schemas`, `api`, `viewer`.

```bash
objectiveai agents list
objectiveai agents spawn --agent remote=github,owner=...,repository=... --inline '...'
objectiveai functions executions create standard --function remote=github,owner=...,repository=... --profile remote=github,owner=...,repository=... --input-inline '{...}'
objectiveai plugins install github --owner ObjectiveAI --repository my-plugin
```

The default build embeds the Tauri viewer as a sidecar: running a streaming command opens a live viewer window backed by an in-process HTTP server. Pass `--no-viewer` at install time for a smaller build without the embedded viewer. JSON schemas for every public type are accessible at `objectiveai schemas list` / `objectiveai schemas output <name>`.

### `objectiveai-api`

Standalone HTTP API server. Run it with:

```bash
objectiveai-api
```

Key environment variables (all optional):

| Variable | Default | Effect |
|---|---|---|
| `ADDRESS` | `0.0.0.0` | Bind address |
| `PORT` | `5000` | Bind port |
| `OBJECTIVEAI_ADDRESS` | `https://api.objectiveai.dev` | Upstream ObjectiveAI address when proxying |
| `OBJECTIVEAI_AUTHORIZATION` | — | Bearer token for the ObjectiveAI API |
| `OPENROUTER_AUTHORIZATION` | — | Bearer token for OpenRouter |
| `GITHUB_AUTHORIZATION` | — | GitHub token for resource retrieval |
| `MCP_AUTHORIZATION` | — | Bearer token for outbound MCP calls |

The server is streaming-first: every layer (agent completions, vector completions, function executions) produces a typed stream of chunks and yields immediately to the HTTP response — nothing is buffered in the hot path.

### `objectiveai-viewer`

Standalone Tauri desktop application. Presents the same UI that the CLI embeds as a sidecar, but runs as a first-class window manager process rather than being spawned in-process by a CLI command. Reach for it when you want the viewer always open and decoupled from CLI invocations.

### `objectiveai-mcp`

MCP (Model Context Protocol) server built from `objectiveai-mcp`. Exposes ObjectiveAI's tooling over the streamable-HTTP MCP transport so editors and agents (Claude, Cursor, etc.) can invoke it via the standard MCP protocol. Defaults to `0.0.0.0:3000`; override with `ADDRESS` and `PORT`.

Three crates make up the MCP surface:

- **`objectiveai-mcp`** — the primary MCP surface. Wraps the CLI as MCP tools over streamable-HTTP. What users run locally and expose upstream for distributed agents.
- **`objectiveai-mcp-proxy`** — a multiplexing sidecar of `objectiveai-api`. Terminates an MCP client connection and forwards tool calls to an upstream MCP server or to ObjectiveAI-native tools. Embedded inside `objectiveai-api` at runtime.
- **`objectiveai-mcp-filesystem`** — MCP filesystem helpers (read/write/list) adapting the SDK's filesystem layer to MCP tool calls.

### Install flags

Pass flags to `bash -s --` after the installer URL:

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash -s -- --no-viewer
```

| Flag | Effect |
|---|---|
| `--no-viewer` | Skips the standalone `objectiveai-viewer`; installs the CLI variant without an embedded Tauri viewer (smaller binary). |
| `--no-api` | Skips `objectiveai-api`. |
| `--no-mcp` | Skips `objectiveai-mcp`. |
| `--cli-only` | Equivalent to `--no-viewer --no-api --no-mcp`. Only `objectiveai` is installed. |

Flags compose freely.

### Self-host vs hosted

The hosted API at `https://api.objectiveai.dev` requires no setup and is the default for the CLI and all SDKs. Run your own `objectiveai-api` when you need total control over data routing — for example, to point agents at private upstream providers not available on OpenRouter, to meet on-prem or air-gapped requirements, or to run the full execution pipeline locally without network egress. Configure the CLI to point at your instance with `objectiveai api mode set local` and `objectiveai api local address set http://localhost:5000`.

Supported platforms: Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64 (Apple Silicon), Windows x86_64.

## Plugins

A plugin is a binary that adds new top-level subcommands to the ObjectiveAI CLI, optionally paired with a viewer UI tab. Plugins are described by an `objectiveai.json` manifest at the repository root. The CLI dispatches any unknown top-level subcommand to the matching installed plugin binary, communicating over a JSONL protocol on stdout. The viewer surfaces plugins with a declared UI source as sandboxed iframe tabs, isolated from the host DOM.

### Installing a plugin

Install from a public GitHub repository:

```bash
# From the ObjectiveAI org (default whitelist — no extra flags needed).
objectiveai plugins install github --owner ObjectiveAI --repository my-plugin

# Pin to a specific commit.
objectiveai plugins install github --owner ObjectiveAI --repository my-plugin --commit-sha <sha>

# Third-party repository — requires explicit opt-in.
objectiveai plugins install github --owner third-party --repository my-plugin --allow-untrusted

# Replace an existing install (binary, viewer bundle, and manifest are rewritten).
objectiveai plugins install github --owner ObjectiveAI --repository my-plugin --upgrade
```

To print layout and manifest conventions for placing a plugin by hand in `~/.objectiveai/plugins/`:

```bash
objectiveai plugins install filesystem
```

### Plugin manifest

`objectiveai.json` at the repository root declares the plugin's metadata, platform binaries, and optional viewer source. All fields except `description` and `version` are optional.

| Field | Type | Notes |
|---|---|---|
| `description` | string | Required. One-line summary shown in listings. |
| `version` | string | Required. Used to construct release-asset URLs (`releases/download/v<version>/<asset>`). |
| `author` / `homepage` / `license` | string | Optional metadata. |
| `binaries` | object | Map of `<os>_<arch>` → release-asset filename. Supported keys: `linux_x86_64`, `linux_aarch64`, `windows_x86_64`, `windows_aarch64`, `macos_x86_64`, `macos_aarch64`. Declare only platforms you ship. |
| `viewer_zip` | string | Release-asset filename for the UI bundle (a zip with `index.html` at root). Mutually exclusive with `viewer_url`. |
| `viewer_url` | string | Remote URL loaded as the iframe `src` verbatim. Must be `https://` or `http://localhost`. Mutually exclusive with `viewer_zip`. |
| `viewer_routes` | array | HTTP routes the viewer's embedded axum server exposes on behalf of the plugin. |
| `mobile_ready` | bool | Opt-in for iOS/Android viewer builds. Defaults to false. |

Example:

```json
{
  "description": "Run wave-physics simulations from the CLI.",
  "version": "1.0.0",
  "author": "Example Corp",
  "license": "MIT",
  "binaries": {
    "linux_x86_64":   "psyops-linux-x86_64",
    "windows_x86_64": "psyops-windows-x86_64.exe",
    "macos_aarch64":  "psyops-macos-aarch64"
  },
  "viewer_zip": "psyops-viewer.zip"
}
```

### Building a plugin

A plugin binary reads its arguments from `argv` and writes JSONL to stdout. Each line must be one of three shapes:

```jsonc
{"type": "notification", "key": "value"}        // data to forward to the caller
{"type": "error", "level": "warn", "fatal": false, "message": "..."}
{"type": "command", "command": "agents list"}    // spawn a CLI command, fire-and-forget
```

The host parses stdout line-by-line; unparseable lines are forwarded as string notifications rather than dropped.

For the viewer, produce a static `dist/` with `index.html` at the root, zip it, and reference it in `viewer_zip`. For remote-hosted UIs, use `viewer_url`. The viewer posts events to the iframe via `postMessage`.

To iterate locally: place the binary at `~/.objectiveai/plugins/<name>/plugin[.exe]` and the manifest at `~/.objectiveai/plugins/<name>.json`, then invoke `objectiveai <name> <args>`. The `objectiveai-cli/test-fixtures/hello-plugin/` fixture is the minimal example — a single `main.rs` that reads `argv[1]` and emits one notification line.

For distribution, cut a GitHub release tagged `v<version>`, upload binaries and the viewer zip as release assets named exactly as declared in the manifest, then install with `plugins install github`.

Full reference: [PLUGINS.md](PLUGINS.md).

## Web app & ecosystem

### Web app

[objectiveai.dev](https://objectiveai.dev) is the production web interface, built with Next.js (App Router). The app provides browsing and detail views for the three core resource types: Functions (`/functions`, `/{owner}/{repo}`), Swarms (`/swarms`, `/{id}`), and Profiles (`/profiles`). From a function detail page, users can inspect the task tree, execute the function against a chosen swarm and profile, and view per-task vote breakdowns and aggregate scores. The profiles listing surfaces trained weight configurations available for reuse. A `/demo` route renders live component prototypes including the `FunctionTree` canvas visualization, vote matrices, decomposition views, and contribution waterfalls.

### Examples

The [`examples/`](examples/) directory collects real software built on top of ObjectiveAI, with links to full source repositories.

**[psychological-operations](examples/psychological-operations.md)** — an agentic X (Twitter) scraper and scoring pipeline ([repo](https://github.com/WiggidyW/psychological-operations)). It pairs human-driven Chrome automation with ObjectiveAI to rank scraped tweets along operator-defined axes. The project defines two primary objects: *Scrapes* (declarative search jobs that scroll and parse `x.com` into SQLite) and *PsyOps* (scoring jobs that pull tagged posts and run them through an ObjectiveAI function using a chosen swarm, profile, and strategy — including Swiss System tournament-style ranking). A pilot study ranked tweets from 33 YC W22 CEO accounts along an *unsettlingness* axis; published artifacts are content-addressed and reproducible.

### Ecosystem

- **`objectiveai-claude-agent-sdk-runner`** — a long-lived Python stdio NDJSON server that runs concurrent Claude Agent SDK sessions on behalf of `objectiveai-api`. The Rust API caller spawns and multiplexes requests over a single stdin/stdout pair using a semaphore-backed FIFO queue; each request carries a string `id` for demultiplexing events from N concurrent streams.
- **`objectiveai-codex-sdk-runner`** — same architecture as the Claude runner but targets the OpenAI Codex SDK. Authentication is inherited from `~/.codex/auth.json`; the runner shells out to the `codex` binary and streams `ThreadEvent` objects back to the Rust caller.
- **`objectiveai-function-tree`** — a TypeScript/React package that renders a 2D canvas visualization of ObjectiveAI function execution trees. Exposes a `FunctionTree` component plus a headless `core` export and CSS; peer-depends on React 18+. Used internally by `objectiveai-web`.
- **`objectiveai-cocoindex`** ([PyPI](https://pypi.org/project/objectiveai-cocoindex/)) — a Python integration that wraps ObjectiveAI function executions as memoized [CocoIndex](https://github.com/cocoindex-io/cocoindex) processing components. The memo key combines the bound `(function, profile, strategy)` triple with the per-call input, making it safe to drop into indexing pipelines.
- **`objectiveai-github-discord-notifier`** — a Python FastAPI webhook server (Docker-deployable) that validates GitHub webhook signatures and forwards pull-request and issue events to a configured Discord channel.
- **`objectiveai-json-schema`** — generated JSON Schema files for every public serializable type in the Rust SDK, named using dot-separated module paths (e.g. `functions.executions.RetryToken.json`). Several hundred schemas cover agents, swarms, functions, profiles, executions, CLI output, MCP types, and more. These files drive code generation for the Go SDK and .NET SDK and can be used by any downstream tooling that needs machine-readable type definitions.

## Repository structure

A single git repository contains the SDK core, server, clients, integrations, and tools.

```text
objectiveai/
│
├── # SDK core (Rust)
│   ├── objectiveai-sdk-rs/                    # Rust SDK — types, validation, compilation
│   ├── objectiveai-sdk-rs-macros/             # Procedural macros for the Rust SDK
│   ├── objectiveai-sdk-rs-cffi/               # C FFI bindings (expose SDK to C/C++)
│   ├── objectiveai-sdk-rs-pyo3/               # PyO3 bindings (Rust extension for Python)
│   └── objectiveai-sdk-rs-wasm-js/            # WASM bindings for browser / Node.js
│
├── # SDKs (other languages)
│   ├── objectiveai-sdk-js/                    # TypeScript/JavaScript SDK (npm)
│   ├── objectiveai-sdk-py/                    # Python SDK (PyPI)
│   ├── objectiveai-sdk-go/                    # Go SDK
│   └── objectiveai-dotnet/                    # .NET SDK (NuGet: ObjectiveAI)
│
├── # Server & binaries
│   ├── objectiveai-api/                       # API server (self-hostable or importable)
│   ├── objectiveai-cli/                       # Command-line interface
│   ├── objectiveai-viewer/                    # Desktop viewer app (Tauri)
│   └── objectiveai-mcp/                       # MCP server binary (ships as objectiveai-mcp)
│
├── # MCP integration
│   ├── objectiveai-mcp-proxy/                 # MCP proxy — multiplexes tool calls
│   └── objectiveai-mcp-filesystem/            # MCP filesystem helpers
│
├── # Runners
│   ├── objectiveai-claude-agent-sdk-runner/   # Concurrent Claude Agent SDK runner
│   └── objectiveai-codex-sdk-runner/          # Concurrent OpenAI Codex SDK runner
│
├── # Web & tools
│   ├── objectiveai-web/                       # Next.js production web interface
│   ├── objectiveai-function-tree/             # 2D canvas function-tree visualizer
│   ├── objectiveai-cocoindex/                 # CocoIndex integration (Python)
│   ├── objectiveai-github-discord-notifier/   # GitHub webhook → Discord notifier
│   └── objectiveai-json-schema/               # Generated JSON Schema files
│
└── # Other
    ├── examples/                              # Usage examples
    ├── bin/                                   # Vendored build tool binaries
    └── *.sh                                   # Root scripts: build, install, publish, version
```

## Contributing & development

### Prerequisites

- **Rust** — stable toolchain via [rustup](https://rustup.rs/). No pinned `rust-toolchain.toml`; use the current stable release. `wasm-pack` and `maturin` are installed automatically into `./bin/` by `build.sh` (its first step).
- **Node.js + pnpm 10.25.0** — the workspace `packageManager` field pins this version. Install pnpm via `corepack enable` or `npm i -g pnpm@10.25.0`.
- **Python** — required for `objectiveai-sdk-py` (PyO3/maturin extension build) and the Claude/Codex agent-SDK runners (PyInstaller).

### Build

```bash
pnpm install                 # JS workspace dependencies
cargo build --release        # Rust crates
bash build.sh                # full monorepo build in dependency order
                             # (first installs pinned build tools into ./bin/)
```

`build.sh` generates JSON schemas, compiles WASM and CFFI bindings, builds all language SDKs (.NET, Go, Python, JS), and produces viewer artifacts.

### Test

```bash
bash test.sh                 # all suites in parallel (spawns a local API server)
cargo test                   # Rust workspace tests
pnpm test                    # JS/TS tests
```

`test.sh` exports `OBJECTIVEAI_TEST_PORT` and runs per-package `test.sh` scripts concurrently across `objectiveai-sdk-rs`, `objectiveai-api`, `objectiveai-json-schema`, `objectiveai-cli`, `objectiveai-mcp-proxy`, `objectiveai-sdk-js`, `objectiveai-sdk-py`, `objectiveai-sdk-go`, and `objectiveai-viewer`. Tests must not hit the production API — use the local server, mocks, or fixtures.

### Conventions

- **Package manager:** use `pnpm`, never `npm`. Filter to a single workspace package with `pnpm --filter <package-name> run <script>`.
- **No type re-exports in Rust.** When an import path is wrong, fix it at the call site. Never add re-export aliases or shim `pub use` entries to paper over a broken import.
- **`mod.rs` discipline.** `mod.rs` files contain only module declarations and re-export globs — no functions, structs, enums, traits, or impls. Every entry must be either `pub mod foo;` or `mod foo; pub use foo::*;`.
- **No network-hitting tests.** Tests must not contact the production API. Mock responses or use local fixtures.
- **Test failures are not pre-existing issues.** Every failure must be investigated and fixed; never dismiss one to move on.
- **Single shared version.** All packages share one version number. Bump atomically across Cargo.toml, package.json, pyproject.toml, .csproj, and all inter-package dependency references with `bash version.sh <new-version>`.
- **Publishing.** `bash publish.sh` orchestrates the full release across crates.io, PyPI, npm, the Go module proxy, and GitHub Releases in dependency-order waves, polling each registry until the new version is live before proceeding.

## License

[MIT](LICENSE).
