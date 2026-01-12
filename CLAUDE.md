# ObjectiveAI Project Notes

**Last Updated:** 2026-01-12 (Updated with GitHub-hosted Functions and Profiles)

## Project Overview

ObjectiveAI is an open-source REST API platform for **scoring, ranking, and simulating preferences** using ensembles of large language models (LLMs). The project motto is: **"Score everything. Rank everything. Simulate anyone."**

**Website:** https://objective-ai.io
**API:** https://api.objective-ai.io
**Repository:** https://github.com/ObjectiveAI/objectiveai

## Core Philosophy

Instead of asking a single model for an answer, ObjectiveAI:
1. Uses **ensembles of LLMs**
2. Applies **explicit, user-controlled weights**
3. Produces **structured outputs** (scores or vectors of scores)
4. Learns weights from example data to simulate real-world preferences

## Repository Structure

This monorepo contains:

```
objectiveai/
├── objectiveai/                          # Main Rust SDK (core implementation)
│   ├── objectiveai/                      # Core Rust crate
│   │   └── src/
│   │       ├── auth/                     # Authentication (API keys)
│   │       ├── chat/                     # Chat completions
│   │       ├── ensemble/                 # Ensemble definitions
│   │       ├── ensemble_llm/             # Ensemble LLM definitions
│   │       ├── functions/                # Function scoring pipelines
│   │       ├── vector/                   # Vector completions
│   │       ├── http/                     # HTTP client (optional feature)
│   │       ├── error.rs                  # Error types
│   │       └── prefixed_uuid.rs          # UUID utilities
│   └── objectiveai-wasm-js/              # WASM bindings for JavaScript
├── objectiveai-js/                       # JavaScript/TypeScript SDK (npm package)
├── objectiveai-web/                      # Next.js web interface
└── objectiveai-github-discord-notifier/  # Discord notifier tool
```

## Core Concepts

### 1. Ensemble LLM

An **Ensemble LLM** is a fully-specified configuration of a single upstream language model:
- Model identity
- Prompt structure (prefix/suffix messages)
- Decoding parameters (temperature, top_p, etc.)
- Provider preferences
- Output mode, reasoning settings, verbosity

**Key Property:** Ensemble LLMs are **content-addressed** - their identity is derived from their full definition, not from a mutable name.

### 2. Ensemble

An **Ensemble** is a collection of Ensemble LLMs used together.

Important properties:
- **Immutable** - any change produces a new Ensemble ID
- **Do NOT contain weights** - weights are execution-time parameters
- Can be defined inline or referenced by ID
- The same Ensemble supports many different behaviors via different weightings

### 3. Weights

Weights are **execution-time parameters** that control each model's influence:
- External to Ensembles
- Explicit and visible
- Static per request
- Optimizable via training
- Never baked into Ensemble definitions

### 4. Deterministic Identity

A key design goal is **reproducibility**:
- Core objects use **deterministic, content-based identifiers**
- No hidden mutation
- No "latest version" ambiguity
- No environment-dependent drift
- IDs can be computed anywhere (server, client, browser)

If two definitions are identical, their IDs will always match.

## Three Main API Capabilities

### 1. Chat Completions

Standard LLM chat completions (messages in, text out) with a twist:
- A "model" can be a **configured model definition** (Ensemble LLM)
- May include built-in prefix/suffix messages
- Decoding and provider settings
- Other fixed configuration

Use for: Text output with reusable, shareable model configurations.

### 2. Vector Completions

Produces **numbers**, not text:
- Takes a prompt AND possible responses (text, images, videos, files, audio)
- Runs **multiple Chat Completions** (one per LLM in an Ensemble)
- Forces each completion to produce a **vote** for a user-defined response
- Combines votes using explicit **weights**
- Returns a **vector of scores** that sums to 1

Use for:
- Picking a winner
- Ranking options
- Producing machine-usable scoring outputs

Intentionally low-level and powerful.

### 3. Functions (Primary Interface)

**Functions are what most users use.**

A Function is a **composable scoring pipeline** built from Vector Completions (and other Functions):

**Data in → Score(s) out**

Key properties:
- Accepts structured input
- Executes a list of tasks
- Each task is either:
  - A Vector Completion, or
  - Another Function
- Produces either:
  - A single score (scalar), or
  - A vector of scores
- Can be very simple or complex (multi-stage decision trees)
- Public, reusable, immutable, and versioned

Use for: "Give me a score", "rank these", "classify this document"

#### GitHub-Hosted Functions

**Functions are now hosted on GitHub repositories** (as of recent architectural changes).

**How it works:**
- Users reference a Function by specifying `owner/repo-name`
- ObjectiveAI retrieves the repository and looks for `function.json` at the repository root
- The `function.json` file contains the complete Function definition

**Benefits:**
- **Decentralized hosting** - No central registry required; anyone can publish Functions
- **Git-based versioning** - Functions leverage Git's branching, tagging, and commit history
- **Standard collaboration** - Teams use Pull Requests, issues, and forks
- **Discoverability** - Functions can be searched, forked, and improved community-wide
- **Immutability via commits** - Specific Function versions can be referenced by Git SHA

**Example:**
```
Reference: "objectiveai/sentiment-scorer"
ObjectiveAI fetches: https://github.com/objectiveai/sentiment-scorer/function.json
```

## Training Functions (Profiles)

ObjectiveAI does **not** fine-tune LLMs. Instead, it:
1. Keeps LLMs fixed
2. Learns **weights** over ensembles

To train a Function, provide:
- A dataset of inputs
- The desired output for each input (score, winner, or target vector)

ObjectiveAI:
- Executes the Function repeatedly
- Computes loss
- Adjusts weights
- Caches AI outputs (optimization becomes CPU-bound)
- Produces a **Profile**

A **Profile** is a learned configuration that makes a Function behave as desired.

Profiles are:
- Immutable
- Versioned
- Shareable
- Reusable across compatible Functions

#### GitHub-Hosted Profiles

**Profiles are also hosted on GitHub repositories**, similar to Functions.

**How it works:**
- Users reference a Profile by specifying `owner/repo-name`
- ObjectiveAI retrieves the repository and looks for `profile.json` at the repository root
- The `profile.json` file contains the learned weights and Profile configuration

**Benefits:**
- **Sharing trained weights** - Trained Profiles can be published and reused across projects
- **Version control** - Profile training iterations tracked through Git history
- **Reproducibility** - Exact Profile state can be referenced via Git commit SHA or tag
- **Community collaboration** - Users can contribute improved Profiles for shared Functions
- **Transparency** - Profile weights and training metadata are publicly accessible
- **A/B testing** - Different Profile versions can be compared using Git branches

**Example:**
```
Reference: "objectiveai/sentiment-scorer-profile"
ObjectiveAI fetches: https://github.com/objectiveai/sentiment-scorer-profile/profile.json
```

**Relationship to Functions:**
- A Profile is typically paired with a specific Function (or Function family)
- Profiles contain weights that work with specific Ensemble configurations
- Multiple Profiles can exist for the same Function, trained on different datasets
- Users can specify both Function and Profile by their respective GitHub references

## Technology Stack

### Rust SDK (`objectiveai` crate)
- **Language:** Rust (Edition 2024)
- **Key Dependencies:**
  - `serde` - Serialization/deserialization
  - `indexmap` - Ordered maps
  - `serde_json` - JSON support with order preservation
  - `twox-hash` - Fast hashing (xxhash3_128)
  - `rust_decimal` - Precise decimal arithmetic
  - `jmespath` - Expression evaluation
  - `base62`, `base64` - Encoding
  - `chrono` - Date/time handling
  - `uuid` - UUID generation
  - **Optional HTTP feature:**
    - `reqwest` - HTTP client
    - `reqwest-eventsource` - SSE support
    - `futures` - Async streams

### WASM Bindings (`objectiveai-wasm-js`)
- Compiles Rust core to WebAssembly for browser/JS environments
- Uses `wasm-bindgen` for JS interop
- Enables client-side ID computation and validation

### JavaScript SDK (`objectiveai` npm package)
- **Version:** 1.2.5
- **Language:** TypeScript
- **Key Dependencies:**
  - `zod` - Runtime type validation and schema definitions
  - `openai` - OpenAI SDK compatibility (dev dependency)
- Provides both ESM and CommonJS builds
- Published to npm as `objectiveai`

### Web Interface (`@objectiveai/web`)
- **Framework:** Next.js 15 (with Turbopack)
- **UI:** React 19 with Tailwind CSS 4
- **Authentication:** NextAuth.js
- **Payments:** Stripe integration
- **Key Features:**
  - Drag-and-drop UI (`@dnd-kit`)
  - Markdown rendering with syntax highlighting
  - Date handling
  - JSON schema support
  - Uses the `objectiveai` npm package (v1.2.5)

## Design Principles

1. **Immutability over mutation**
2. **Explicit configuration over hidden defaults**
3. **Structural validation over assumptions**
4. **Reproducibility over convenience**
5. **Composition over monoliths**

These constraints enable large-scale scoring, ranking, and simulation.

## Browser & Client Compatibility

This repository includes bindings that allow Ensemble and Ensemble LLM IDs to be computed **outside the ObjectiveAI backend**, including in browser environments.

Enables:
- UI-based Ensemble builders
- Client-side validation
- Previewing and sharing configurations
- Identical ID computation across environments

## What's NOT in This Repository

Intentionally excluded:
- Backend execution engines
- Optimization logic
- Storage implementations
- Product-specific APIs

Those live elsewhere.

## Use Cases

ObjectiveAI is designed for:
- Ranking candidates, items, or options
- Scoring quality, relevance, similarity, or preference
- Simulating human or organizational decision-making
- Building scoring/ranking infrastructure
- Integrating AI-powered scoring into existing systems
- **Publishing and sharing** Functions and Profiles via GitHub (new)
- **Discovering and reusing** community-created scoring pipelines (new)

## Target Audience

Useful for:
- Developers building tools on top of ObjectiveAI
- Teams integrating ObjectiveAI into existing systems
- Engineers designing scoring, ranking, or preference infrastructure

**Note:** You do NOT need to be an ML specialist to use these primitives.

## Current Branch Status

**Branch:** `add-rs-sdk`
**Status:** Clean working directory

Recent commits:
- Remove 'objectiveai-ensemble' (now part of 'objectiveai' root)
- Flatten structure, no modules
- Fix WASM target
- Add client-side compilation of function tasks and function output
- Remove http feature from WASM build

## Module Organization

### Rust Core Modules
- `auth` - API authentication (API keys, HTTP requests)
- `chat` - Chat completion APIs and models
- `ensemble` - Ensemble definitions and responses
- `ensemble_llm` - Ensemble LLM configurations (output modes, providers, reasoning, stop sequences, verbosity)
- `functions` - Function definitions, profiles, tasks, expressions
  - `compute_profile` - Profile computation logic
  - `executions` - Function execution tracking
  - `expression` - JMESPath expression evaluation
- `vector` - Vector completion APIs
- `http` - HTTP client implementation (feature-gated)
- `error` - Error types and handling
- `prefixed_uuid` - Prefixed UUID utilities

### JavaScript SDK Structure
The JavaScript SDK (`objectiveai-js`) provides comprehensive TypeScript definitions with Zod schemas for:
- Expressions (JMESPath)
- Messages (text, image, video, file, audio content)
- Ensemble LLMs and Ensembles
- Chat, Vector, and Function APIs
- Profiles and training
- Full type safety and runtime validation

## Key Insights

1. **Content-Addressed Everything:** All core objects (Ensemble LLMs, Ensembles, Functions) use deterministic IDs based on their content, ensuring reproducibility.

2. **Weights Are Separate:** Unlike traditional ensemble methods, weights are NOT part of the ensemble definition. This allows the same ensemble to be used with different weight configurations.

3. **Composition First:** Functions can call other Functions, creating pipelines and decision trees. This compositional approach enables building complex scoring systems from simple building blocks.

4. **No Fine-Tuning:** ObjectiveAI doesn't fine-tune models. Instead, it learns optimal weights over fixed models, making training much faster and more interpretable.

5. **Cross-Environment:** The same ID computation works in Rust, JavaScript (Node), and browsers (WASM), enabling seamless cross-platform tooling.

6. **HTTP Feature Flag:** The Rust SDK's HTTP client is optional (`http` feature), allowing use as a pure data structure library without network dependencies.

7. **GitHub-Hosted Functions and Profiles:** Functions and Profiles are hosted on GitHub repositories as `function.json` and `profile.json` files. This decentralized approach leverages Git for versioning, collaboration, and discoverability, making ObjectiveAI a truly open ecosystem where anyone can publish and share scoring pipelines.

## Future Directions

The README notes:
> "Ultimately, you do not even need to have any idea what's happening. Claude will be the world's foremost expert at using the ObjectiveAI SDK. Turn it on and watch the magic happen."

This suggests the project aims to make AI-powered scoring and ranking accessible to all developers, with AI assistants (like Claude) providing expert guidance.
