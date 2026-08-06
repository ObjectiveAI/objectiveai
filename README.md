# ObjectiveAI

**The Swarm Harness.**

Define an agent once — model, prompts, tools, MCP servers — then spawn it to do work, or hand it a Docker sandbox to act in. From the CLI or your own agent.

[Website](https://objectiveai.dev) · [Discord](https://discord.gg/gbNFHensby) · [GitHub](https://github.com/ObjectiveAI/objectiveai)

[![Release](https://img.shields.io/github/v/release/ObjectiveAI/objectiveai?label=release&color=blue)](https://github.com/ObjectiveAI/objectiveai/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Binaries

Install the prebuilt binaries with one command:

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
export PATH="$HOME/.objectiveai/bin:$PATH"
```

| Binary | What it does | Download |
|---|---|---|
| `objectiveai` | CLI + embedded viewer | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |
| `objectiveai-api` | API server | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |
| `objectiveai-viewer` | Standalone Tauri desktop app | [latest](https://github.com/ObjectiveAI/objectiveai/releases/latest) |

Supported platforms: Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64, Windows x86_64. See [Binaries & self-hosting](#binaries--self-hosting) for per-binary detail.

## Scaffolding a plugin

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

See [Plugins](#plugins) for what a plugin is, the manifest reference, and the first-party ones.

---

## What ObjectiveAI is

ObjectiveAI is a harness for defining and running agents — distributed across the CLI, the API, the MCP server, and your own agents. You define an **Agent** once — model, prompts, decoding parameters, output mode, tools, MCP servers — and then run it: spawn it to do work, or hand it a Docker sandbox to act in.

Agents are content-addressed, Git-hosted resources. The same `agent.json` that powers your CLI invocation tonight is the one your colleague pins by commit SHA next month.

The mechanism is the **Agent**: a reusable, composable, version-tracked configuration of a model. Everything else (the CLI, the API, the web app, the MCP server) exists to drive agents in the ways that matter.

### Why this system

Reusability. Content-addressing throughout:

- **Reusable.** An Agent is a 22-character ID — define one once and reference it from anywhere. Run it for action or for sandboxed work without re-defining anything.
- **Reproducible.** Every resource reference is `(owner, repo, commit)`. Pin a commit SHA, get the exact same agent your run used six months ago.
- **Composable.** Agents call other agents. The CLI dispatches plugins as unknown subcommands. The viewer surfaces plugin UIs as sandboxed iframe tabs.

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

Pin a `commit=<sha>` segment to lock in a specific version of any remote resource. See [Core primitives](#core-primitives) for a full explanation of Agents and agent completions.

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

CLI: `objectiveai agents spawn --agent remote=github,owner=...,repository=... --inline '...'`. Executions stream typed chunks over Server-Sent Events.

### Resource resolution

Resources are referenced by `(owner, repository, commit)` triple. Content-addressing plus commit pinning makes any execution reproducible from its request alone.

Remote references resolve lazily: the retrieval system fetches and caches each resource exactly once, deduplicating by triple. All fetches are content-verified — a cached resource is never re-fetched if the commit SHA matches.

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

## License

[MIT](LICENSE).
