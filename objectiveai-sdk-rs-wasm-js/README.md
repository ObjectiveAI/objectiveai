# objectiveai-sdk-rs-wasm-js

WebAssembly bindings for ObjectiveAI, providing JavaScript/TypeScript access to client-side validation and compilation.

## Overview

This crate compiles Rust code from `objectiveai-rs` to WebAssembly, enabling browser-based applications to:

- Validate Swarm LLM and Swarm configurations
- Compute content-addressed IDs (deterministic hashes using XXHash3-128)
- Compile Function expressions for previewing during authoring
- Compute prompt, tools, and response IDs for caching/deduplication

## Exported Functions

| Function | Description |
|----------|-------------|
| `validateSwarmLlm(llm)` | Validates an Swarm LLM configuration and computes its content-addressed ID |
| `validateSwarm(swarm)` | Validates an Swarm configuration and computes its content-addressed ID |
| `compileFunctionTasks(function, input)` | Compiles a Function's task expressions for a given input |
| `compileFunctionOutput(function, input, taskOutputs)` | Computes the final output of a Function given input and task results |
| `promptId(prompt)` | Computes a content-addressed ID for chat messages |
| `toolsId(tools)` | Computes a content-addressed ID for a tools array |
| `vectorResponseId(response)` | Computes a content-addressed ID for a vector completion response option |

## Usage

This crate is consumed via the `objectiveai-sdk` npm package. The TypeScript SDK wraps these functions with proper type definitions.

```typescript
import { validateSwarmLlm, validateSwarm } from 'objectiveai';

// Validate and get ID for an Swarm LLM
const validatedLlm = validateSwarmLlm({
  model: 'openai/gpt-4o',
  temperature: 0.7,
});

// Validate and get ID for an Swarm
const validatedSwarm = validateSwarm({
  llms: [validatedLlm],
});
```

## Building

Requires [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/):

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build for bundler (webpack, etc.)
wasm-pack build --target bundler

# Build for Node.js
wasm-pack build --target nodejs

# Build for web (no bundler)
wasm-pack build --target web
```

## Development

```bash
# Run tests
cargo test

# Build documentation
cargo doc --no-deps --open
```

## License

See the LICENSE file in this directory.
