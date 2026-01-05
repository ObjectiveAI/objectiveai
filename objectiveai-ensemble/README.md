# objectiveai-ensemble

Rust types + validation utilities for describing an “ensemble” of LLM configurations, plus optional WebAssembly/JavaScript bindings.

This repository is a Cargo workspace with two crates:

- `objectiveai-ensemble` (Rust library)
- `objectiveai-ensemble-js` (wasm-bindgen bindings, published as `@objectiveai/ensemble`)

## Workspace layout

- `objectiveai-ensemble/` — core Rust types and validation
- `objectiveai-ensemble-js/` — WebAssembly bindings exposing a small validation API

## Rust crate: `objectiveai-ensemble`

### What it provides

At a high level:

- `EnsembleLlmBase` — the user-provided LLM configuration shape
- `EnsembleLlm` — validated form (adds a deterministic `id`)
- `EnsembleBase` — the user-provided ensemble shape (`llms: Vec<...>`)
- `Ensemble` — validated form (adds a deterministic `id`, normalizes/merges entries)

Validation happens through `TryFrom` conversions:

- `EnsembleLlmBase -> EnsembleLlm`
- `EnsembleBase -> Ensemble`

Both conversions use `type Error = String`.

## JS/WASM crate: `objectiveai-ensemble-js`

This crate exposes validation helpers to JavaScript via `wasm-bindgen`.

### Exported functions

From `objectiveai-ensemble-js/src/lib.rs`:

- `validateEnsembleLlm(llm: any) -> any`
  - Parses a JS object into `EnsembleLlmBase`, validates it, converts to `EnsembleLlm`, then returns a JS object containing the validated structure (including `id`).
- `validateEnsemble(ensemble: any) -> any`
  - Parses a JS object into `EnsembleBase`, validates/normalizes it, converts to `Ensemble`, then returns a JS object containing the validated structure (including `id`).

On failure, these throw/return a JS error value (a string message).
