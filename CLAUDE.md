# ObjectiveAI

**Score everything. Rank everything. Simulate anyone.**

ObjectiveAI is a REST API platform for scoring, ranking, and simulating preferences using ensembles of LLMs. Instead of asking one model for an answer, it uses multiple LLMs with explicit weights to produce structured numeric outputs.

**API:** https://api.objective-ai.io

## Repository Structure

```
objectiveai/
├── objectiveai-rs/                 # Rust SDK (core crate: data structures, validation, compilation)
├── objectiveai-api/                # API server (self-hostable, or import as library)
├── objectiveai-rs-wasm-js/         # WASM bindings for browser/Node.js
├── objectiveai-js/                 # TypeScript SDK (npm: objectiveai)
└── objectiveai-web/                # Next.js web interface
```

## Core Concepts

### Ensemble LLM

A fully-specified configuration of a single upstream LLM:
- Model identity (e.g., `openai/gpt-4o`, `google/gemini-3.0-pro`)
- Prompt structure (prefix/suffix messages)
- Decoding parameters (temperature, top_p, etc.)
- Output mode (`instruction`, `json_schema`, `tool_call`)
- Provider preferences, reasoning settings

**Content-addressed:** IDs are computed deterministically from the definition using XXHash3-128. Identical definitions always produce identical IDs.

### Ensemble

A collection of Ensemble LLMs used together for voting. Key properties:
- **Immutable** - any change produces a new ID
- **Does NOT contain weights** - weights are execution-time parameters
- Can be defined inline or referenced by pre-computed ID

### Weights

Execution-time parameters controlling each LLM's influence. Weights are:
- External to Ensembles (same Ensemble can behave differently with different weights)
- Learnable via training (Profiles)
- Never baked into definitions

## API Capabilities

### Chat Completions

Standard LLM chat (messages in, text out). The "model" can be an Ensemble LLM ID or inline definition with pre-configured settings.

### Vector Completions

The core primitive. Produces **scores**, not text:

1. Takes a prompt and possible **responses** (text, images, videos, files, audio)
2. Runs Chat Completions across all LLMs in an Ensemble
3. Each LLM **votes** for one of the responses
4. Votes are combined using **weights** to produce **scores**
5. Returns a vector of scores that sums to 1

**Important distinctions:**
- `weights` vector: Same length as `responses`. Shows total weight allocated to each response option.
- `scores` vector: Same length as `responses`. Final normalized scores (sum ≈ 1).
- For discrete votes, an LLM's full weight goes to its selected response.
- For probabilistic votes, weight is divided according to the distribution.

**Probabilistic Voting via Logprobs:**

LLMs are inherently probabilistic - the sampler makes the final discrete choice. ObjectiveAI bypasses the sampler using **logprobs** to capture the model's full preference distribution. Instead of getting one answer and losing confidence signals, we extract probabilities for each option simultaneously.

The prefix tree (`pfx.rs` in objectiveai-api) structures responses around logprobs limits. Tree width matches the number of logprobs returned (typically 20), enabling voting over hundreds of options while preserving probability information at each level. For large response sets, nested prefixes (e.g., `` `A` `` `` `B` ``) capture preferences in stages.

### Functions

Composable scoring pipelines. **Data in → Score(s) out.**

Functions execute a list of **tasks**, where each task is either:
- A Vector Completion
- Another Function (scalar or vector)

Functions produce either:
- **Scalar**: Single score in [0, 1]
- **Vector**: Array of scores that sum ≈ 1

#### GitHub-Hosted Functions

Functions are hosted on GitHub as `function.json` at repository root. Reference by `owner/repo` (optionally with commit SHA for immutability).

### Profiles

Learned weights for Functions. ObjectiveAI doesn't fine-tune LLMs—it learns optimal **weights** over fixed models.

Training provides:
- Dataset of inputs with expected outputs
- ObjectiveAI executes repeatedly, computes loss, adjusts weights
- Produces a Profile (learned weight configuration)

Profiles are also GitHub-hosted as `profile.json`. Highly recommended to specify commit SHA since the profile's shape may change in future versions.

## Expression System (Complex)

Functions use **JMESPath** expressions for dynamic behavior. This is the most complex part of the SDK.

### Input Maps

`input_maps` transforms input into a **2D array** for mapped task execution:

```json
{
  "input_maps": [
    {"$jmespath": "input.items"},
    {"$jmespath": "input.categories"}
  ]
}
```

Each sub-array can be referenced by tasks via their `map` index.

### Task Expressions

Each task can have:

- **`skip`**: Boolean expression. If true, task is skipped.
  ```json
  {"$jmespath": "input.count < 10"}
  ```

- **`map`**: Index into `input_maps`. Creates **multiple task instances**—one per element in that sub-array. Each instance is compiled with a different value from the 1D array. The number of compiled tasks equals the length of that sub-array.

- **`input`**: Expression defining task input from function input and map context.

### Output Expression

Computes final result from input and task outputs:

```json
{"$jmespath": "tasks[0].output"}
```

**Constraints:**
- Scalar functions: output must be in [0, 1]
- Vector functions: output must sum ≈ 1 and match `output_length` if specified

### Expression Context

Available variables during compilation:
- `input`: Original function input
- `tasks`: Array of task outputs (indexed, null if skipped)
- `map`: Current map element (only in mapped task context)

### Compilation Methods

Client-side compilation is for **previewing during authoring**, not required before execution:

- **`compile_tasks(input)`**: Shows what final tasks look like for given input
- **`compile_output(input, task_outputs)`**: Shows final output given input and task results
- **`compile_one()`**: Expects exactly one value. OK for single value or 1-element array. Error for null, empty array, or multi-element array.

**Validation checks:**
- Output type matches function type (scalar vs vector)
- Scalar outputs in [0, 1]
- Vector outputs sum ≈ 1
- Vector outputs match `output_length` if specified

## Usage Response Fields

**Cost fields (important distinction):**
- `cost`: What ObjectiveAI charged for this request
- `total_cost`: Sum of all costs including upstream providers (only differs with BYOK)
- `cost_details`: Breakdown showing `upstream` and `upstream_upstream` costs

**Reasoning field:**
- `models`: Fallback models tried in order if primary is rate-limited or errors

## Rust SDK

Location: `objectiveai-rs/`

**Capabilities:**
1. Data structures and validation
2. Deterministic ID computation (XXHash3-128, base62 encoded)
3. Client-side Function compilation (JMESPath evaluation)
4. HTTP client (optional `http` feature, enabled by default)

**Key modules:**
- `auth/` - API key authentication
- `chat/completions/` - Chat request/response types
- `vector/completions/` - Vector completion types, votes, scores
- `ensemble/` - Ensemble validation and ID computation
- `ensemble_llm/` - Ensemble LLM configuration and validation
- `functions/` - Function definitions, tasks, profiles, expressions
- `http/` - HTTP client (feature-gated)

## API Server

Location: `objectiveai-api/`

Self-hostable API server. Can be run locally or imported as a library to build custom servers.

**Modules:**
- `auth/` - Authentication client
- `chat/completions/` - Chat completions with backoff and streaming
- `vector/completions/` - Vector completions orchestration, probabilistic voting, caching
- `functions/` - Function execution, flattening, Profile computations
- `ensemble/` and `ensemble_llm/` - Fetching and caching
- `ctx/` - Request context for dependency injection (enables BYOK)

**Key implementation details:**
- `pfx.rs` - Prefix tree for logprobs-based probabilistic voting
- `flat_task_profile.rs` - Flattens nested Function + Profile trees for parallel execution
- `completion_votes_fetcher/` - Fetches votes from actual LLM inference (excludes cached/RNG votes, includes retries)

**Running locally:**
```bash
cd objectiveai-api
OPENROUTER_API_KEY=sk-or-... cargo run --release
```

## WASM Bindings

Location: `objectiveai-rs-wasm-js/`

Exports:
- `validateEnsembleLlm(llm)` - Validate and compute Ensemble LLM ID
- `validateEnsemble(ensemble)` - Validate and compute Ensemble ID
- `compileFunctionTasks(function, input)` - Compile tasks client-side
- `compileFunctionOutput(function, input, taskOutputs)` - Compile output client-side

Enables browser-based validation and preview without server round-trips.

## Design Principles

1. **Content-addressed identities** - Reproducibility via deterministic IDs
2. **Weights separate from definitions** - Same Ensemble, different behaviors
3. **Composition** - Functions call Functions, building complex pipelines from simple parts
4. **No fine-tuning** - Learn weights over fixed models (faster, more interpretable)
5. **Cross-environment** - Same logic in Rust, Node.js, and browsers
