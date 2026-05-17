# {ai} | ObjectiveAI

[![npm version](https://img.shields.io/npm/v/objectiveai-sdk.svg)](https://www.npmjs.com/package/objectiveai-sdk)
[![Crates.io](https://img.shields.io/crates/v/objectiveai-sdk.svg)](https://crates.io/crates/objectiveai-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## The agentic collective judgment harness.

Your agent doesn't have to decide alone. ObjectiveAI lets any agent call out to a **swarm** of models for collective judgment — routing decisions through recursive scoring trees that map arbitrary input into a vector of scores across every option. Functions are invented by agents, trained on data, and deployed as reusable decision infrastructure that gets better over time.

[Website](https://objectiveai.dev) | [API](https://api.objectiveai.dev) | [Discord](https://discord.gg/gbNFHensby) | [npm](https://www.npmjs.com/package/objectiveai-sdk) | [crates.io](https://crates.io/crates/objectiveai-sdk) | [Built with ObjectiveAI](examples/README.md)

## What this is

An agent faces a choice. Instead of relying on a single model's judgment, it calls an ObjectiveAI Function — a recursive decision tree hosted as a JSON file. The Function fans out to a swarm of models, each with its own perspective. They vote. Their votes combine with learned weights. Out comes a vector of scores that sums to 1, one per option. The agent takes the highest-scoring option and moves on.

The scoring pipeline itself is a composition of tasks — vector completions, nested functions, map operations — that can be arbitrarily deep. A Function can call other Functions. An agent can *invent* new Functions. The whole system is content-addressed, version-tracked, and trainable.

```
Agent has a decision
    -> Calls ObjectiveAI Function
        -> Function fans out to swarm of models
            -> Each model votes across all options
            -> Votes combine with learned weights
        -> Returns scores: [0.42, 0.31, 0.18, 0.09]
    -> Agent takes the best option
```

## Install

### Binaries

Install all four prebuilt binaries with one command:

```bash
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
. "$HOME/.objectiveai/env"
```

What lands in `~/.objectiveai/`:

| Binary | What it is |
|---|---|
| `objectiveai` | CLI — the user-facing command (`objectiveai agents list`, `objectiveai functions executions create`, etc.) with the embedded Tauri viewer. |
| `objectiveai-api` | Standalone API server — runs the `objectiveai-api` HTTP server in the foreground; `OBJECTIVEAI_PORT=8080 objectiveai-api`. |
| `objectiveai-viewer` | Standalone Tauri desktop app — the same viewer the CLI embeds, but launchable on its own. |
| `objectiveai-mcp` | MCP (Model Context Protocol) server — wraps the CLI as an MCP tool so editors / agents can invoke it via the streamable-HTTP MCP transport. |

Pick a subset via flags (compose freely):

```bash
# CLI without the embedded Tauri viewer + api + mcp, skip standalone viewer
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash -s -- --no-viewer

# Skip the standalone API server (keep CLI + viewer + mcp)
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash -s -- --no-api

# Skip the MCP server (keep CLI + api + viewer)
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash -s -- --no-mcp

# Just the CLI, no embedded viewer, no api, no standalone viewer, no mcp
curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash -s -- --cli-only
```

Sourcing `~/.objectiveai/env` puts the binaries on `PATH` for the current shell. New shells pick it up automatically (the installer wires `~/.bashrc` / `~/.zshrc` to source the same file).

Supported platforms: Linux x86_64, Linux aarch64 (Raspberry Pi 4+, Graviton), macOS x86_64, macOS aarch64 (Apple Silicon), Windows x86_64. All four binaries are published per-platform on [GitHub Releases](https://github.com/ObjectiveAI/objectiveai/releases). The CLI self-updates on startup; re-run the installer to upgrade `objectiveai-api` / `objectiveai-viewer` / `objectiveai-mcp`.

### SDK

```bash
npm install objectiveai-sdk
```

```toml
[dependencies]
objectiveai-sdk = "2.0.7"
```

## Core primitives

### Agents

An **Agent** is a fully-specified configuration of a single upstream model — model identity, prompt structure, decoding parameters, output mode, provider preferences. Content-addressed via XXHash3-128, so the same configuration always produces the same ID.

Agents are stored as `agent.json` in Git repositories. Reference them by `owner/repo@commit` or define them inline.

### Swarms

A **Swarm** is a collection of Agents used together for collective judgment. Each agent can have its own personality, temperature, output mode, and count. Weights control each agent's influence on the final score.

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
  ],
  "weights": [0.6, 0.4]
}
```

Swarms are stored as `swarm.json` in Git repositories — shareable, version-tracked, and reusable across Functions.

### Vector Completions

The core primitive. Give a swarm a prompt and a set of possible responses. Each agent votes for what it thinks is the best response. Votes combine with weights to produce a score vector that sums to 1.

```
Prompt: "Which approach best handles edge cases?"
Responses: ["defensive coding", "fuzzing", "formal verification", "property testing"]

-> Scores: [0.12, 0.28, 0.19, 0.41]
```

#### Probabilistic voting via logprobs

LLMs are inherently probabilistic — the sampler makes the final discrete choice, destroying information. ObjectiveAI bypasses the sampler entirely using **logprobs** to capture each model's full preference distribution.

If a model is 70% confident in option A and 30% in option B, we capture both signals rather than losing one to sampling. For large response sets exceeding logprobs limits, a prefix tree captures preferences in stages — the tree width matches the logprobs count (typically 20), enabling voting over hundreds of options while preserving probability information at each level.

```
Traditional: Model outputs "A" (loses the 30% signal for B)
ObjectiveAI: Model vote = [0.70, 0.30, 0.00, 0.00] (full distribution preserved)
```

### Functions

**Functions** are composable scoring pipelines. Data in, scores out. They're recursive decision trees that can contain vector completions, nested function calls, and map operations — arbitrarily composed.

```
Input -> [Task 1: Vector Completion] -> Score
         [Task 2: Nested Function]   -> Score
         [Task 3: Mapped Function]   -> Score
      -> Weighted average -> Final score
```

Functions are hosted as `function.json` in Git repositories. Reference them by `owner/repo`:

```
objectiveai/sentiment-scorer
```

Functions produce either a **scalar** (single score in [0, 1]) or a **vector** (array of scores summing to 1). A scalar function that calls a vector function that calls another scalar function — all valid, all composable.

### Profiles

ObjectiveAI doesn't fine-tune models. It learns **weights**.

Give it a dataset of inputs with expected outputs. ObjectiveAI executes repeatedly, computes loss, and adjusts the weights across your swarm to match. The result is a **Profile** — a learned weight configuration stored as `profile.json` that makes your Function's judgments converge on ground truth.

### The resource graph

Resources reference each other inline or remote:

```
agent.json  <-  swarm.json  <-  profile.json      function.json
                 (agents)        (swarms+weights)   (tasks + input_schema)

At execution: function.json + profile.json  ->  scores
```

Function and profile are independent files — execution takes both and combines them. All remote references use `(owner, repository, commit)` triples. Pin a commit SHA for reproducibility, or omit it to resolve to latest. The retrieval system resolves the entire graph, caching and deduplicating fetches along the way.

## Function invention

Agents can **invent** new Functions. The invention system takes a description of what you want to score, generates the input schema, designs the task tree, and produces a complete `function.json` — ready to deploy, train, and use. Recursive invention builds multi-level decision trees where each node is itself an invented Function.

An agent that can invent its own judgment criteria, train them on data, and deploy them for future use. That's the loop.

## Concepts

| Concept | What it is |
|---------|-----------|
| **Agent** | A configured model with prompt, params, output mode. Content-addressed. `agent.json`. |
| **Swarm** | A collection of Agents with weights. `swarm.json`. |
| **Vector Completion** | Prompt + responses -> score vector that sums to 1. |
| **Function** | Recursive scoring pipeline. Data in, scores out. `function.json`. |
| **Profile** | Learned weights for a Function. Trained on data. `profile.json`. |
| **Invention** | Agent-driven creation of new Functions. |

## Repo structure

```
objectiveai/
├── objectiveai-rs/             # Rust SDK (core crate: types, validation, compilation)
├── objectiveai-api/            # API server (self-hostable)
├── objectiveai-cli/            # CLI tool
├── objectiveai-mcp-cli/        # MCP server exposing the CLI as an MCP tool
├── objectiveai-mcp-filesystem/ # MCP filesystem server (Docker-injected into lab executions)
├── objectiveai-mcp-proxy/      # MCP multiplexing proxy (sidecar of objectiveai-api)
├── objectiveai-viewer/         # Local Tauri viewer (optional, embedded in CLI)
├── objectiveai-web/            # Web interface (production)
├── objectiveai-js/             # TypeScript SDK (npm: objectiveai)
├── objectiveai-rs-wasm-js/     # WASM bindings for browser/Node.js
├── objectiveai-py/             # Python package
├── objectiveai-rs-pyo3/        # Rust crate behind objectiveai-py
├── objectiveai-go/             # Go SDK
├── objectiveai-dotnet/         # .NET SDK (in progress)
├── objectiveai-rs-cffi/        # C FFI bindings (foundation for other langs)
└── objectiveai-json-schema/    # Generated JSON Schema files
```

## Related

### [ObjectiveAI-claude-code-1](https://github.com/ObjectiveAI-claude-code-1)

An autonomous Claude Code agent that invents and publishes ObjectiveAI Functions without human intervention. Uses the Agent SDK to create, test, and deploy new scoring pipelines.

## License

MIT
