# ObjectiveAI

**The Swarm Harness.**

Define an agent once — model, prompts, tools, MCP servers — then spawn it to do work, or hand it a Docker sandbox to act in. From the CLI, the SDKs, or your own agent.

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

Additional crates on crates.io: [`objectiveai-api`](https://crates.io/crates/objectiveai-api), [`objectiveai-cli`](https://crates.io/crates/objectiveai-cli), [`objectiveai-mcp`](https://crates.io/crates/objectiveai-mcp), [`objectiveai-mcp-proxy`](https://crates.io/crates/objectiveai-mcp-proxy), [`objectiveai-mcp-laboratory`](https://crates.io/crates/objectiveai-mcp-laboratory), [`objectiveai-sdk-macros`](https://crates.io/crates/objectiveai-sdk-macros).

## Binaries

Install all four prebuilt binaries with one command:

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
export PATH="$HOME/.objectiveai/bin:$PATH"
```

| Binary | What it does | Download |
|---|---|---|
| `objectiveai` | CLI + embedded viewer | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |
| `objectiveai-api` | API server | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |
| `objectiveai-viewer` | Standalone Tauri desktop app | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |

Supported platforms: Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64, Windows x86_64. See [Binaries & self-hosting](#binaries--self-hosting) for install flags and per-binary detail.

---

## What ObjectiveAI is

ObjectiveAI is a harness for defining and running agents — distributed across the CLI, the API, the SDKs, the MCP server, and your own agents. You define an **Agent** once — model, prompts, decoding parameters, output mode, tools, MCP servers — and then run it: spawn it to do work, or hand it a Docker sandbox to act in.

Agents are content-addressed, Git-hosted resources. The same `agent.json` that powers your CLI invocation tonight is the one your colleague pins by commit SHA next month.

The mechanism is the **Agent**: a reusable, composable, version-tracked configuration of a model. Everything else (the CLI, the API, the web app, the MCP server, the SDKs in five languages) exists to drive agents in the ways that matter.

### Why this system

Reusability. Content-addressing throughout:

- **Reusable.** An Agent is a 22-character ID — define one once and reference it from anywhere. Run it for action or for sandboxed work without re-defining anything.
- **Reproducible.** Every resource reference is `(owner, repo, commit)`. Pin a commit SHA, get the exact same agent your run used six months ago.
- **Composable.** Agents call other agents. The CLI dispatches plugins as unknown subcommands. The viewer surfaces plugin UIs as sandboxed iframe tabs.
- **Polyglot.** Rust, TypeScript, Python, Go, and (in-progress) .NET SDKs share the same generated JSON Schema corpus. Field names and shapes are identical across languages.

## Quick start

Install the CLI, API server, viewer, and MCP server from the latest release:

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
export PATH="$HOME/.objectiveai/bin:$PATH"
```

Set your API key:

```bash
objectiveai api headers x-objectiveai-authorization config set "apk_your_key_here"
```

### CLI — spawn an agent to do work

```bash
objectiveai agents spawn \
  --agent remote=github,owner=your-org,repository=writer-agent \
  --inline '[{"role":"user","content":"Write a haiku about ocean waves."}]'
```

Pin a `commit=<sha>` segment to lock in a specific version of any remote resource. See [Core primitives](#core-primitives) for a full explanation of Agents and agent completions, and [SDKs](#sdks) for Python, Rust, Go, and .NET patterns including streaming.

### SDK — TypeScript

```typescript
import { ObjectiveAI, agentsCompletionsCreateAgentCompletion } from "@objectiveai/sdk";

const client = new ObjectiveAI({ authorization: process.env.OBJECTIVEAI_AUTHORIZATION });

const result = await agentsCompletionsCreateAgentCompletion(client, {
  agent: { remote: "github", owner: "your-org", repository: "writer-agent" },
  messages: [{ role: "user", content: "Write a haiku about ocean waves." }],
  stream: false,
});

console.log(result);
```

## Core primitives

One **resource** (the Agent) defines what's in the system; one **execution mode** (the agent completion) defines what you can do with it. Resources are content-addressed Git-hosted JSON; an execution resolves them at request time and streams typed results back.

### Agents

An **Agent** is a fully-specified configuration of a single upstream model: model identity, prompt structure, decoding parameters, output mode, tools, MCP servers, provider preferences. Agents are content-addressed via XXHash3-128 — the same configuration always produces the same 22-character base62 ID. IDs are deterministic because the serialized configuration is hashed after normalization (empty fields stripped, defaults canonicalized). Two Agents with identical effective settings are the same Agent.

Agents are stored as `agent.json` in Git repositories and referenced by `owner/repo@commit` everywhere an agent is needed. Authoring agents lives in source control; calling them happens by reference.

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

Each upstream (OpenRouter, Claude Agent SDK, Codex SDK) has its own agent type with its own parameter set.

### Agent completions

An **agent completion** spawns a single Agent to do work. The Agent receives a task as a conversation and acts on it — calls tools, talks to MCP servers, executes a multi-turn loop, writes code, generates artifacts.

The Agent is supplied by remote reference. Messages can include images, audio, and files in addition to text. Tool calls are detected mid-stream and executed automatically; MCP servers attached to the Agent are dialed transparently. The response carries a `Continuation` that captures the conversation state so the next call can pick up where this one left off.

```json
{
  "agent": { "remote": "github", "owner": "your-org", "repository": "writer-agent" },
  "messages": [
    { "role": "user", "content": "Rewrite this commit message as a conventional-commits changelog entry." }
  ]
}
```

CLI: `objectiveai agents spawn --agent remote=github,owner=...,repository=... --inline '...'`. SDK: `agentsCompletionsCreateAgentCompletion` (JS) / `create_agent_completion` (Python) / `agent::completions::http::create_agent_completion` (Rust). Executions stream typed chunks over Server-Sent Events.

### Resource resolution

Resources are referenced by `(owner, repository, commit)` triple. Content-addressing plus commit pinning makes any execution reproducible from its request alone.

Remote references resolve lazily: the retrieval system fetches and caches each resource exactly once, deduplicating by triple. All fetches are content-verified — a cached resource is never re-fetched if the commit SHA matches.

## SDKs

Every SDK exposes **Agent Completions** — spawn a single Agent to do work, with tools, MCP, and multi-turn loops — with streaming via Server-Sent Events. The API emits incremental chunks; each SDK merges them into an accumulating object using an immutable merge system (TypeScript), a mutable push system (Python, Rust, Go), or equivalent. Types are generated from a shared JSON Schema corpus derived from the Rust SDK, so field names and shapes are identical across languages.

### Languages

| Language | Package | Install | Runtime targets |
|---|---|---|---|
| Rust | `objectiveai-sdk` on crates.io | `cargo add objectiveai-sdk` | Any (async via `reqwest` + `tokio`) |
| TypeScript | `@objectiveai/sdk` on npm | `npm i @objectiveai/sdk` | Node.js, Deno, browser (CJS + ESM) |
| Python | `objectiveai-sdk` on PyPI | `pip install objectiveai-sdk` | CPython 3.10+ (includes PyO3 extension) |
| Go | `github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go` | `go get github.com/ObjectiveAI/objectiveai/objectiveai-sdk-go` | Go 1.26+ |
| .NET | `ObjectiveAI` (NuGet — in progress) | not yet published | net10.0 |

### Streaming examples

The base URL defaults to `https://api.objectiveai.dev` in all SDKs. Auth is passed as `OBJECTIVEAI_AUTHORIZATION` (env var) or via the client constructor. Each example spawns an agent with `stream: true` and consumes the streamed chunks.

#### TypeScript

```typescript
import { ObjectiveAI, agentsCompletionsCreateAgentCompletion } from "@objectiveai/sdk";

const client = new ObjectiveAI({ authorization: process.env.OBJECTIVEAI_AUTHORIZATION });

const stream = await agentsCompletionsCreateAgentCompletion(client, {
  stream: true,
  agent: { remote: "github", owner: "your-org", repository: "writer-agent" },
  messages: [{ role: "user", content: "Write a haiku about ocean waves." }],
});

for await (const chunk of stream) {
  process.stdout.write(JSON.stringify(chunk) + "\n");
}
```

#### Python

```python
import asyncio, os
from objectiveai_sdk.client import ObjectiveAI
from objectiveai_sdk.agent.completions.http import create_agent_completion

async def main() -> None:
    client = ObjectiveAI(authorization=os.environ.get("OBJECTIVEAI_AUTHORIZATION"))
    params = {
        "stream": True,
        "agent": {"remote": "github", "owner": "your-org", "repository": "writer-agent"},
        "messages": [{"role": "user", "content": "Write a haiku about ocean waves."}],
    }
    stream = await create_agent_completion(client, params)
    acc = None
    async for chunk in stream:
        if acc is None:
            acc = chunk
        else:
            acc.push(chunk)
    print("output:", acc)

asyncio.run(main())
```

#### Rust

```rust
use futures::StreamExt;
use objectiveai_sdk::{HttpClient, agent::completions};

#[tokio::main]
async fn main() -> Result<(), objectiveai_sdk::HttpError> {
    let client = HttpClient::builder()
        .authorization(std::env::var("OBJECTIVEAI_AUTHORIZATION").ok())
        .build();

    let mut stream = completions::http::create_agent_completion_streaming(
        &client,
        completions::request::params(/* agent: remote ref, messages */),
    ).await?;

    while let Some(Ok(chunk)) = stream.next().await {
        println!("{chunk:?}");
    }
    Ok(())
}
```

### Go and .NET

The Go SDK is fully auto-generated from the JSON Schema corpus. Types are strict-validated on unmarshal. The client exposes generic helpers `PostUnary[T]` / `PostStreaming[T]` / `GetUnary[T]` / `DeleteUnary[T]`; endpoint functions wrap these. A wazero-hosted WASM binary (compiled from the Rust core) provides chunk-to-unary conversion and merge verification without CGO.

The .NET SDK (`ObjectiveAI`, targeting net10.0) is in active development. The NuGet publish workflow is not yet wired up, so it must be built from source for now.

## Binaries & self-hosting

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
export PATH="$HOME/.objectiveai/bin:$PATH"
```

All binaries land in `~/.objectiveai/bin/` and are added to `PATH`. The CLI (`objectiveai`) self-updates on startup; re-run the installer to upgrade `objectiveai-api` and `objectiveai-viewer`.

### `objectiveai` (CLI)

The primary user-facing binary. Built with `clap` derive macros and emits newline-delimited JSON (NDJSON) on stdout. Top-level command groups: `agents`, `laboratories`, `channels`, `tasks`, `development`, `daemon`, `db`, `api`, `viewer`, `python`, `update`.

```bash
objectiveai agents list
objectiveai agents spawn --agent remote=github,owner=...,repository=... --inline '...'
objectiveai laboratories spawn
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

The server is streaming-first: every layer produces a typed stream of chunks and yields immediately to the HTTP response — nothing is buffered in the hot path.

### `objectiveai-viewer`

Standalone Tauri desktop application. Presents the same UI that the CLI embeds as a sidecar, but runs as a first-class window manager process rather than being spawned in-process by a CLI command. Reach for it when you want the viewer always open and decoupled from CLI invocations.

### MCP (served by the daemon)

The daemon itself serves MCP (Model Context Protocol) over streamable HTTP at `/mcp` on its own address, executing commands in-process. Editors and agents (Claude, Cursor, etc.) point at `http://127.0.0.1:<daemon-port>/mcp` (plus the `X-OBJECTIVEAI-SIGNATURE` header when a daemon secret is configured).

Three crates make up the MCP surface:

- **the daemon's `/mcp` route** — the primary MCP surface. Wraps the CLI as MCP tools over streamable HTTP, in-process. What users expose upstream for distributed agents.
- **`objectiveai-mcp-proxy`** — a multiplexing sidecar of `objectiveai-api`. Terminates an MCP client connection and forwards tool calls to an upstream MCP server or to ObjectiveAI-native tools. Embedded inside `objectiveai-api` at runtime.
- **`objectiveai-mcp-laboratory`** — MCP filesystem helpers (read/write/list) adapting the SDK's filesystem layer to MCP tool calls.

### Install flags

Pass flags to `bash -s --` after the installer URL:

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash -s -- --no-viewer
```

| Flag | Effect |
|---|---|
| `--no-viewer` | Skips the standalone `objectiveai-viewer`; installs the CLI variant without an embedded Tauri viewer (smaller binary). |
| `--no-api` | Skips `objectiveai-api`. |
| `--cli-only` | Equivalent to `--no-viewer --no-api`. Only `objectiveai` is installed. |

Flags compose freely.

### Self-host vs hosted

The hosted API at `https://api.objectiveai.dev` requires no setup and is the default for the CLI and all SDKs. Run your own `objectiveai-api` when you need total control over data routing — for example, to point agents at private upstream providers not available on OpenRouter, to meet on-prem or air-gapped requirements, or to run the full execution pipeline locally without network egress. Configure the CLI to point at your instance with `objectiveai api mode set local` and `objectiveai api local address set http://localhost:5000`.

Supported platforms: Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64 (Apple Silicon), Windows x86_64.

## Plugins

A plugin extends ObjectiveAI with tools an agent can call, and optionally with UI in the viewer. It is a **container**: an MCP server built from a `Containerfile` in your repository, run as an ephemeral laboratory container for the completion that uses it. A plugin may also ship a **viewer half** — tabs, channel-request handlers, and scripts injected into browser tabs — from the same repository, under the same identity.

Both halves are declared by one `objectiveai.json` at the repository root.

An agent uses a plugin by declaring its coordinates:

```json
{ "plugins": [{ "owner": "you", "name": "my-plugin", "version": "v0.1.0" }] }
```

The laboratory host fetches that GitHub repository at the `v`-prefixed tag, builds the image, and starts a container per completion. Its tools reach the agent prefixed with the MCP server's name — a tool called `greet` arrives as `my-plugin_greet`.

### First-party plugins

Built and maintained by ObjectiveAI:

- **[psychological-operations](https://github.com/ObjectiveAI/psychological-operations)** — run autonomous persona agents on X (Twitter) and Discord. Each agent is an X account plus a Discord bot, addressed by a tag, with tool-mediated presence on both platforms (the `x` and `discord` MCP servers), ranked ingestion pipelines ("psyops" that pull posts/messages, rank them, and deliver the survivors to agents' work queues), and event-driven wake-ups from a resident daemon that fires when an agent is mentioned, replied to, or DM'd.
- **[mundus-animarum](https://github.com/ObjectiveAI/mundus-animarum)** — persistent, self-authored "souls" for agents. A key/value store keyed by an agent's content-addressed ID, with cross-agent lookups, subscriptions, and change notifications; every instance of the same agent definition shares one soul, which the agent can rewrite over time.
- **[arcanum](https://github.com/ObjectiveAI/arcanum)** — skills for agents. Lets agents load skills and governs which agents may use which skills.
- **[quas-wex-exort](https://github.com/ObjectiveAI/quas-wex-exort)** — programmatic invocation of MCP tools and the ObjectiveAI CLI from within an agent, including running them as **background tasks** (create / list / wait / cancel) and batched multi-calls.

### Scaffolding a new plugin

`scaffold.sh` lays down a complete plugin — both halves, one manifest — into the directory you run it from. The plugin's **name is the directory's name**, so that is the only thing you choose up front:

```bash
mkdir my-plugin && cd my-plugin
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/scaffold.sh | bash -s -- rust
```

You get:

```text
my-plugin/
├── objectiveai.json   # the manifest — both halves, at the root
├── README.md
├── mcp/               # the MCP server (Rust): the tools an agent calls
├── viewer/            # tabs, channel handlers, browser scripts
└── .agents/skills/    # skills for a coding agent working on the plugin
```

The name is written into the Cargo package, the binary, the lockfile, the `Containerfile`, the MCP server's `NAME` constant, and `package.json`. `rust` (or `rs`) is currently the only MCP language.

Then register **the root** — one directory, both halves — and start the viewer's watch build:

```bash
objectiveai laboratories spawn                       # once per machine; never auto-started

objectiveai development plugins mcp create \
  --owner you --name my-plugin --version v0.1.0 --path "$PWD"
objectiveai development plugins viewer create \
  --owner you --name my-plugin --version v0.1.0 --path "$PWD"

cd viewer && pnpm install && pnpm run dev
```

An agent declaring `you/my-plugin@v0.1.0` now gets your working tree instead of a git tag. After editing the MCP half, `development plugins mcp reset …` — a registered plugin still takes the image-exists fast path, so without it the old image keeps serving. The viewer half needs no reset: the watch build writes, and the viewer reloads open tabs.

To release, tag `vX.Y.Z`, push, and delete the registrations.

### The manifest

`objectiveai.json` at the repository root. At least one of `mcp` / `viewer` must be present; a plugin may ship either or both.

| Field | Type | Notes |
|---|---|---|
| `description` | string | One-line summary. |
| `mcp.containerfile` | string | Repo-relative path (forward slashes). **The file's own directory is the build context**, so `mcp/Containerfile` sees `mcp/` as its root. |
| `mcp.port` | number | The port the server listens on inside the container. Must match `PORT` in the server and `EXPOSE` in the `Containerfile`. |
| `mcp.postgres` | bool | Required. Opts the plugin in to its own database; only then is `OBJECTIVEAI_POSTGRES_URL` set in the container. |
| `mcp.development.caches` | string[] | CONTAINER paths kept between development rebuilds (e.g. `/build/target`). Ignored for a released plugin. |
| `viewer.containerfile` | string | As above; its own directory is the build context. |
| `viewer.output` | string | Absolute path INSIDE the built image whose contents are the built assets. |
| `viewer.tabs` | array | `{ title, module, styles }` for a normal tab, or `{ channel_key, module, styles }` for a channel-request handler. `module`/`styles` are relative to the built output. |
| `viewer.scripts` | array | `{ name, module }` — classic scripts a tab can inject into a browser tab it spawns. |
| `viewer.development.output` | string | HOST path, relative to the REGISTERED directory, where the watch build writes (`viewer/dist` in the scaffolded layout). |

```json
{
  "description": "An ObjectiveAI plugin: a Rust MCP server and a viewer extension.",
  "mcp": {
    "containerfile": "mcp/Containerfile",
    "port": 8080,
    "postgres": true,
    "development": { "caches": ["/build/target", "/usr/local/cargo/registry"] }
  },
  "viewer": {
    "containerfile": "viewer/Containerfile",
    "output": "/dist",
    "tabs": [{ "title": "home", "module": "./home.js", "styles": ["./home.css"] }],
    "scripts": [{ "name": "capture", "module": "./capture.js" }],
    "development": { "output": "viewer/dist" }
  }
}
```

Identity is **not** in the manifest: owner, name and version come from the repository and its tag on release, and from the explicit `--owner/--name/--version` of a development registration otherwise.

## Web app & ecosystem

### Web app

[objectiveai.dev](https://objectiveai.dev) is the production web interface, built with Next.js (App Router). The app provides browsing and detail views for Agents, and lets you run one and watch its output stream back. A `/demo` route renders live component prototypes.

### Examples

The [`examples/`](examples/) directory collects real software built on top of ObjectiveAI, with links to full source repositories.

**[psychological-operations](examples/psychological-operations.md)** — an agentic X (Twitter) scraper and ranking pipeline ([repo](https://github.com/WiggidyW/psychological-operations)). It pairs human-driven Chrome automation with ObjectiveAI to rank scraped tweets along operator-defined axes. The project defines two primary objects: *Scrapes* (declarative search jobs that scroll and parse `x.com` into SQLite) and *PsyOps* (ranking jobs that pull tagged posts and run them through ObjectiveAI). A pilot study ranked tweets from a set of public startup-founder accounts along an *unsettlingness* axis; published artifacts are content-addressed and reproducible.

### Ecosystem

- **`objectiveai-claude-agent-sdk-runner`** — a long-lived Python stdio NDJSON server that runs concurrent Claude Agent SDK sessions on behalf of `objectiveai-api`. The Rust API caller spawns and multiplexes requests over a single stdin/stdout pair using a semaphore-backed FIFO queue; each request carries a string `id` for demultiplexing events from N concurrent streams.
- **`objectiveai-codex-sdk-runner`** — same architecture as the Claude runner but targets the OpenAI Codex SDK. Authentication is inherited from `~/.codex/auth.json`; the runner shells out to the `codex` binary and streams `ThreadEvent` objects back to the Rust caller.
- **`objectiveai-github-discord-notifier`** — a Python FastAPI webhook server (Docker-deployable) that validates GitHub webhook signatures and forwards pull-request and issue events to a configured Discord channel.
- **`objectiveai-json-schema`** — generated JSON Schema files for every public serializable type in the Rust SDK, named using dot-separated module paths. Several hundred schemas cover agents, completions, CLI output, MCP types, and more. These files drive code generation for the Go SDK and .NET SDK and can be used by any downstream tooling that needs machine-readable type definitions.

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
│   └── objectiveai-mcp/                       # retired standalone MCP server (folded into the daemon, #276)
│
├── # MCP integration
│   ├── objectiveai-mcp-proxy/                 # MCP proxy — multiplexes tool calls
│   ├── objectiveai-mcp-laboratory/            # MCP filesystem helpers
│   ├── objectiveai-mcp-plugin-framework-rs/   # Rust framework for writing plugin MCP servers
│   └── objectiveai-db-proxy/                  # Postgres-over-WebSocket conduit injected into plugin containers
│
├── # Plugin scaffolds (what scaffold.sh assembles)
│   ├── objectiveai-plugin-scaffold-rs/        # the shared root: one manifest covering both halves
│   ├── objectiveai-mcp-plugin-scaffold-rs/    # the MCP half (Rust)
│   └── objectiveai-viewer-plugin-scaffold/    # the viewer half (tabs, handlers, scripts)
│
├── # Runners
│   ├── objectiveai-claude-agent-sdk-runner/   # Concurrent Claude Agent SDK runner
│   └── objectiveai-codex-sdk-runner/          # Concurrent OpenAI Codex SDK runner
│
├── # Web & tools
│   ├── objectiveai-web/                       # Next.js production web interface
│   ├── objectiveai-cocoindex/                 # CocoIndex integration (Python)
│   ├── objectiveai-github-discord-notifier/   # GitHub webhook → Discord notifier
│   └── objectiveai-json-schema/               # Generated JSON Schema files
│
└── # Other
    ├── examples/                              # Usage examples
    ├── bin/                                   # Vendored build tool binaries
    ├── .agents/skills/                        # Skills for coding agents working in this repo
    ├── scaffold.sh                            # Scaffold a new plugin into the current directory
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
- **Publishing.** The `Release` GitHub Actions workflow fires on every push to main, gated on the `objectiveai-cli` version: if the GitHub Release `v<version>` doesn't exist yet, it rolls out everything for that version, all-or-nothing — the six per-platform binary zips (each built by `build.sh --release --no-sdk`) plus the language SDKs published sequentially (rust → python → javascript → golang). The SDK jobs ship already-committed artifacts (no codegen, no wasm build), so commit fresh generated artifacts (via `build.sh`) before bumping the version.

## License

[MIT](LICENSE).
