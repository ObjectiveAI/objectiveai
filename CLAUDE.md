# ObjectiveAI

**IMPORTANT: DO NOT RE-EXPORT TYPES.** When fixing import issues, never add re-exports unless explicitly authorized. Fix the import path at the call site instead.

ObjectiveAI is an agentic collective judgment harness. It uses scoring, ranking, and simulation across swarms of agents to produce collective judgments that can be easily fine-tuned. Full brand description is a WIP.

**API:** https://api.objectiveai.dev

## Repository Structure

```
objectiveai/
├── objectiveai-rs/                 # Rust SDK (core crate: data structures, validation, compilation)
├── objectiveai-api/                # API server (self-hostable, or import as library)
├── objectiveai-rs-wasm-js/         # WASM bindings for browser/Node.js
├── objectiveai-js/                 # TypeScript SDK (npm: objectiveai)
├── objectiveai-json-schema/        # Generated JSON Schema files (built from Rust SDK)
├── objectiveai-cli/                # ObjectiveAI CLI
├── objectiveai-web/                # Next.js web interface (production)
├── objectiveai-scripts/            # Utility scripts (see objectiveai-scripts/README.md)
└── coding-agent-scratch/           # Scratch folder for testing SDK calls
```

## Core Concepts

### Agent

A fully-specified configuration of a single upstream model: model identity, prompt structure, decoding parameters, output mode, provider preferences. **Content-addressed:** IDs are deterministic via XXHash3-128.

### Swarm

A collection of Agents used together for collective judgment. Immutable, does NOT contain weights (weights are execution-time parameters). Can be inline or referenced by ID.

### Weights

Execution-time parameters controlling each Agent's influence. External to Swarms, learnable via training (Profiles), never baked into definitions.

## API Capabilities

### Agent Completions

Standard chat (messages in, text out). The "model" can be an Agent ID or inline definition with pre-configured settings.

### Vector Completions

The core primitive. Produces **scores**, not text:

1. Takes a prompt and possible **responses** (text, images, videos, files, audio)
2. Runs Agent Completions across all Agents in a Swarm
3. Each Agent **votes** for one of the responses
4. Votes are combined using **weights** to produce **scores**
5. Returns a vector of scores that sums to 1

**Important distinctions:**
- `weights` vector: Same length as `responses`. Shows total weight allocated to each response option.
- `scores` vector: Same length as `responses`. Final normalized scores (sum ≈ 1).
- For discrete votes, an Agent's full weight goes to its selected response.
- For probabilistic votes, weight is divided according to the distribution.

**Probabilistic Voting via Logprobs:**

Upstream models are inherently probabilistic - the sampler makes the final discrete choice. ObjectiveAI bypasses the sampler using **logprobs** to capture the model's full preference distribution. Instead of getting one answer and losing confidence signals, we extract probabilities for each option simultaneously.

The prefix tree (`pfx.rs` in objectiveai-api) structures responses around logprobs limits. Tree width matches the number of logprobs returned (typically 20), enabling voting over hundreds of options while preserving probability information at each level. For large response sets, nested prefixes (e.g., `` `A` `` `` `B` ``) capture preferences in stages.

### Functions

Composable scoring pipelines. **Data in → Score(s) out.**

Functions execute a list of **tasks**, where each task is either:
- A Vector Completion
- Another Function (scalar or vector)

Functions produce either:
- **Scalar**: Single score in [0, 1]
- **Vector**: Array of scores that sum ≈ 1

### Profiles

Learned weights for Functions. ObjectiveAI doesn't fine-tune models—it learns optimal **weights** over fixed agents.

Training provides:
- Dataset of inputs with expected outputs
- ObjectiveAI executes repeatedly, computes loss, adjusts weights
- Produces a Profile (learned weight configuration)

Profiles are also GitHub-hosted as `profile.json`. Highly recommended to specify commit SHA since the profile's shape may change in future versions.

## Expression System (Complex)

Functions use **expressions** for dynamic behavior. Two languages are supported:
- **JMESPath** (`{"$jmespath": "..."}`) - JSON query language
- **Starlark** (`{"$starlark": "..."}`) - Python-like configuration language (not Turing-complete)

This is the most complex part of the SDK.

### Task Expressions

Each task can have:

- **`skip`**: Boolean expression. If true, task is skipped.
  ```json
  {"$jmespath": "input.count < `10`"}
  ```

- **`map`**: Expression that evaluates to a **count** (integer). Creates that many task instances. Each instance receives `map` as a 0-based integer index (0, 1, 2, ...), NOT the element itself — use the index to look up data from the input.

- **`input`**: Expression defining task input from function input and map context.

### Input Split / Input Merge (Vector Functions)

Vector functions have `input_split` and `input_merge` expressions for Swiss System tournament-style execution:
- **`input_split`**: Splits input into N sub-inputs (one per pool)
- **`input_merge`**: Recombines sub-inputs back into one input after scoring

### Task Output Expressions

Each task has an `output` expression that transforms its raw result into a FunctionOutput. The function's final output is the **weighted average** of all task outputs using profile weights.

```json
{
  "type": "vector.completion",
  "prompt": "...",
  "responses": [...],
  "output": {"$starlark": "output['scores'][0]"}
}
```

**Task output expression receives:**
- For vector completion tasks: `output` is a VectorCompletionOutput (or array if mapped)
- For function tasks: `output` is a FunctionOutput (or array if mapped)

**Constraints:**
- Each task's output must be valid for the parent function's type
- Scalar functions: task outputs must be in [0, 1]
- Vector functions: task outputs must sum ≈ 1 and match `output_length` if specified

### Expression Context

Available variables during compilation:
- `input`: Original function input
- `map`: Current map index, 0-based integer (only in mapped task context)
- `output`: Raw task result (only in task output expressions)

## Rust Coding Conventions

### `mod.rs` Files

`mod.rs` files must contain **only** module declarations and re-export globs — no functions, structs, enums, traits, impls, or other definitions. Every module declaration must be one of:

- **`pub mod foo;`** — the module is public directly.
- **`mod foo; pub use foo::*;`** — the module is private, but its contents are re-exported publicly via a glob.

There is no third option. Every entry in `mod.rs` is either `pub mod` or private `mod` + `pub use`.

## Conventions

### Test Failures

Never dismiss a test failure as a "pre-existing issue" as an excuse to ignore it. If tests fail, investigate and fix them.

### Code Changes

When asked to "standardize" or "apply patterns from X to Y", preserve existing functionality while adopting the visual/structural patterns. Never remove features (like filters, sorts, controls) unless explicitly told to.

### Quality Checks

After declaring work "complete" on multi-page tasks, enumerate all affected pages and verify each one was touched before finishing.

### Testing Guidelines

- **No network-hitting tests:** Tests should not hit the backend API directly
- Tests should mock API responses or use local test data
- Focus on component rendering, user interactions, and state management

## Development Workflow

### Package Manager

This repo uses **pnpm** (not npm). Run all commands with `pnpm`. Workspace filtering uses `pnpm --filter <package-name> run <script>`.

### SDK Testing

Use `coding-agent-scratch/` folder for testing SDK calls and exploring API behavior. Create test scripts to verify CORS, client-side SDK usage, and API responses.

### Merging from Main

**CRITICAL:** Any merge from `main` warrants full understanding of its contents and implications. Before merging:

1. Review all changed files and understand the scope
2. Check for changes to:
   - API contracts (request/response types)
   - SDK methods and signatures
   - WASM bindings exports
   - Design system (globals.css, components)
3. Update this CLAUDE.md if backend capabilities change
4. Test affected web pages after merge
5. Verify no regressions in browse/execute flows
