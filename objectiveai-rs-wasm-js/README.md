# objectiveai-rs-wasm-js

WebAssembly bindings for ObjectiveAI, providing JavaScript/TypeScript access to client-side validation and compilation.

## Overview

This crate compiles Rust code from `objectiveai-rs` to WebAssembly, enabling browser-based applications to:

- Validate Ensemble LLM and Ensemble configurations
- Compute content-addressed IDs (deterministic hashes using XXHash3-128)
- Compile Function expressions for previewing during authoring
- Compute prompt, tools, and response IDs for caching/deduplication

## Exported Functions

| Function | Description |
|----------|-------------|
| `validateEnsembleLlm(llm)` | Validates an Ensemble LLM configuration and computes its content-addressed ID |
| `validateEnsemble(ensemble)` | Validates an Ensemble configuration and computes its content-addressed ID |
| `compileFunctionTasks(function, input)` | Compiles a Function's task expressions for a given input |
| `compileFunctionOutput(function, input, taskOutputs)` | Computes the final output of a Function given input and task results |
| `promptId(prompt)` | Computes a content-addressed ID for chat messages |
| `toolsId(tools)` | Computes a content-addressed ID for a tools array |
| `vectorResponseId(response)` | Computes a content-addressed ID for a vector completion response option |

## Usage

This crate is consumed via the `objectiveai` npm package. The TypeScript SDK wraps these functions with proper type definitions.

```typescript
import { validateEnsembleLlm, validateEnsemble } from 'objectiveai';

// Validate and get ID for an Ensemble LLM
const validatedLlm = validateEnsembleLlm({
  model: 'openai/gpt-4o',
  temperature: 0.7,
});

// Validate and get ID for an Ensemble
const validatedEnsemble = validateEnsemble({
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
