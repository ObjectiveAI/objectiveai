# ObjectiveAI – Open Source

**ObjectiveAI** ([objective-ai.io](https://objective-ai.io)) is a REST API ([api.objective-ai.io](https://api.objective-ai.io)) for **scoring, ranking, and simulating preferences** using ensembles of large language models (LLMs).

This repository contains the **open-source** of ObjectiveAI. It defines ensembles and SDKs.

Specifically, it includes:

- Canonical definitions of **Ensemble LLMs** and **Ensembles**
- Deterministic, content-addressed **ID computation**
- Shared logic used across ObjectiveAI tooling
- Browser-compatible bindings for building and validating ensembles
- Language-agnostic primitives that other ObjectiveAI components build on

---

## What is ObjectiveAI?

ObjectiveAI is built around a simple idea:

> **Score everything. Rank everything. Simulate anyone.**

Instead of asking a single model for an answer, ObjectiveAI:

- Uses **ensembles of LLMs**
- Applies **explicit, user-controlled weights**
- Produces **structured outputs** (scores or vectors of scores)
- Learns those weights from example data to simulate real-world preferences

ObjectiveAI is designed for:

- Ranking candidates, items, or options
- Scoring quality, relevance, similarity, or preference
- Simulating human or organizational decision-making

---

## Core Capabilities

ObjectiveAI exposes three main capabilities via its API.

---

### Chat Completions

Chat Completions are **standard LLM chat completions**: messages in, text out.

The key difference is what "model" means.

In ObjectiveAI, a model can be a **configured model definition** (an _Ensemble LLM_), which may include:

- Built-in prefix or suffix messages (for personality, instructions, or structure)
- Decoding and provider settings
- Other fixed configuration that should travel with the model

From the caller’s perspective, this is still a normal chat completion.
The extra structure is handled automatically by the model definition.

Use Chat Completions when you want text output, but with reusable, shareable model configurations.

---

### Vector Completions

Vector Completions produce **numbers**, not text.

A Vector Completion:

- Takes in not only a prompt, but also possible responses to that prompt, which may include text, images, videos, files, or audio, or all at once.
- Runs **multiple Chat Completions** (one per LLM in an Ensemble)
- Forces each completion to produce a **vote** for a possible user-defined response
- Combines those votes using explicit **weights**
- Returns a **vector of scores** that sums to 1

Vector Completions are used to:

- Pick a winner
- Rank options
- Produce machine-usable scoring outputs

They are powerful, but intentionally low-level.

---

### Functions (What Most Users Use)

**Functions are the primary interface for most users.**

A Function is a **composable scoring pipeline** built from Vector Completions (and other Functions).

Conceptually:

> **Data in -> Score(s) out**

A Function:

- Accepts structured input
- Executes a list of tasks
- Each task is either:
  - a Vector Completion, or
  - another Function
- Produces either:
  - a single score (scalar), or
  - a vector of scores

Functions can be:

- Very simple (one input, one score)
- Complex (multi-stage decision trees)
- Public and reusable
- Immutable and versioned

If your use case is "give me a score" or "rank these" or "classify this document," you want Functions.

---

## Training Functions (Profiles)

Functions can be **trained**.

ObjectiveAI does not fine-tune LLMs. Instead, it:

- Keeps LLMs fixed
- Learns **weights** over ensembles

To train a Function, you provide:

- A dataset of inputs
- The desired output for each input (a score, a winner, or a target vector)

ObjectiveAI:

- Executes the Function repeatedly
- Computes loss
- Adjusts weights
- Caches AI outputs so optimization becomes CPU-bound
- Produces a **Profile**

A Profile is a learned configuration that makes a Function behave the way you want.

Profiles are:

- Immutable
- Versioned
- Shareable
- Reusable across compatible Functions

---

## Core Concepts

### Ensemble LLM

An **Ensemble LLM** is a fully specified configuration of a single upstream language model, including:

- Model identity
- Prompt structure (prefix/suffix messages)
- Decoding parameters
- Provider preferences

Ensemble LLMs are **content-addressed**: their identity is derived from their full definition, not from a mutable name.

---

### Ensemble

An **Ensemble** is a collection of Ensemble LLMs used together.

Important properties:

- Ensembles are **immutable**
- Ensembles do **not** contain weights
- Any change produces a new Ensemble ID
- Ensembles can be defined inline or referenced by ID

The same Ensemble can support many different behaviors via different weightings.

---

### Weights

Weights are **execution-time parameters** that control how much influence each model has.

They are:

- External to Ensembles
- Explicit
- Static per request
- Optimizable

Weights are never baked into Ensemble definitions.

---

## Deterministic Identity

A key design goal of ObjectiveAI is **reproducibility**.

Core objects use **deterministic, content-based identifiers**:

- No hidden mutation
- No "latest version" ambiguity
- No environment-dependent drift
- IDs can be computed anywhere (server, client, browser)

If two definitions are identical, their IDs will always match.

---

## Browser & Client Compatibility

This repository includes bindings that allow Ensemble and Ensemble LLM IDs to be computed **outside the ObjectiveAI backend**, including in browser environments.

This enables:

- UI-based Ensemble builders
- Client-side validation
- Previewing and sharing configurations

ID computation is identical across environments.

---

## Scope of This Repository

This repository contains:

- Open, stable definitions of ObjectiveAI core objects
- Hashing and ID computation logic
- Cross-environment compatible implementations
- Shared primitives used across ObjectiveAI tooling

It intentionally does **not** contain:

- Backend execution engines
- Optimization logic
- Storage implementations
- Product-specific APIs

Those live elsewhere.

---

## Design Principles

ObjectiveAI’s open-source core follows a few simple rules:

- **Immutability over mutation**
- **Explicit configuration over hidden defaults**
- **Structural validation over assumptions**
- **Reproducibility over convenience**
- **Composition over monoliths**

These constraints make large-scale scoring, ranking, and simulation possible.

---

## Who Is This For?

This repository is useful if you are:

- Building tools on top of ObjectiveAI
- Integrating ObjectiveAI into existing systems
- Designing scoring, ranking, or preference infrastructure

You do **not** need to be an ML specialist to use these primitives.

Ultimately, you do not even need to have any idea what's happening. Claude will be the world's foremost expert at using the ObjectiveAI SDK. Turn it on and watch the magic happen.

---

## Learn More

ObjectiveAI is actively evolving. Documentation and tooling will expand as the ecosystem grows.

If you are interested in **ranking, scoring, or simulating decision-making at scale**, ObjectiveAI is the foundation.
