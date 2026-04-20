# objectiveai-js

TypeScript SDK for ObjectiveAI. Published as `objectiveai` on npm.

## Building

`pnpm --filter objectiveai run build` (from the repo root) rebuilds everything, including `objectiveai-rs-wasm-js` (WASM). Do NOT run `wasm-pack` directly — the build script handles it.

## Merge System

The merge system is how streaming chunks are aggregated into a single accumulating object. It is the TypeScript equivalent of `push()` methods in `objectiveai-rs`.

### Why Merge Exists

When streaming responses (function executions, vector completions, agent completions, etc.), the server sends incremental chunks. The client needs to fold each chunk into a running accumulator so that at any point, the consumer has a single coherent object representing the current state. In Rust this is `push(&mut self, other: Self)`. In TypeScript this is `merge(a, b) => [result, changed]`.

### The `[T, boolean]` Return Convention

Every merge function returns `[T, boolean]` — the merged value and whether it changed. The boolean is critical for React integration:

- **React re-renders when object identity changes** (reference equality via `===`).
- If a merge produces no meaningful change, the function returns the **original object reference** (`a`), not a new copy. This means `result === a` and React skips re-rendering that subtree.
- If something did change, a **new object** is created and `changed` is `true`. React sees a new reference and re-renders.
- The root streaming object will almost always change (new chunks keep arriving), but deeply nested objects (e.g., a vote that arrived 10 chunks ago, or scores that haven't updated) will preserve their identity. This means React components rendering those nested pieces won't re-render unnecessarily.

### Core Merge Primitives (`src/merge.ts`)

**`merge<T>(a, b, combine?)`** — The foundational merge function with 4 overloads (T, T|null, T|undefined, T|null|undefined):
- Both present: calls `combine(a, b)` if provided, otherwise returns `[a, false]`
- Only `a` present: returns `[a, false]` (no change, same reference)
- Only `b` present: returns `[b, true]` (new data arrived)
- Both null/undefined: returns `[null/undefined, false]`

**`mergedString(a, b)`** — Concatenates strings incrementally. Empty `b` means no change: `b === "" ? [a, false] : [a + b, true]`

### How to Write a Merge Function

Every streaming chunk type needs a `merged()` function (and list types need `mergedList()`). Follow these rules:

1. **Return `[T, boolean]`** — value + changed flag.
2. **Preserve identity when unchanged** — if no field changed, return `[a, false]` (same reference as input `a`).
3. **Only allocate a new object when something actually changed** — track a `changed` boolean across all field merges; only spread into a new object if `changed` is true.
4. **Use conditional spreading for optional fields** — `...(field !== undefined ? { field } : {})` keeps output shape sparse.
5. **Immutable fields pass through** — fields like `id`, `created`, `index`, `object` come from `a` directly and are never merged.
6. **Compose recursively** — call nested merge functions for nested types. The changed flag bubbles up.

### Field Merge Strategies

These correspond to the patterns used by `push()` in Rust:

| Strategy | TypeScript | Rust `push()` | Used For |
|----------|-----------|---------------|----------|
| **String concat** | `mergedString(a, b)` | `push_option_string()` | `content`, `reasoning`, `refusal` |
| **Replace** | `[b, a !== b]` or deep compare | Direct assignment | `scores`, `weights`, `output`, `state` |
| **Extend list** | Concatenate arrays | `extend_from_slice()` | `votes`, `logprobs` |
| **Index-correlated merge** | Find by `index` field, merge existing, append new | Find by `index`, call `push()` | `tasks`, `completions`, `messages`, `tool_calls` |
| **Lazy set (first wins)** | `merge(a.field, b.field)` with no combiner | `if self.field.is_none() { self.field = other.field }` | `error`, `retry_token`, `finish_reason` |
| **Sum** | `a + b` | `push_option_u64()` / `push_option_decimal()` | Usage token counts, costs |
| **Recursive merge** | `SomeType.merged(a.field, b.field)` | `self.field.push(other.field)` | `usage`, `logprobs`, `reasoning` |

### Index-Correlated List Merging (`mergedList`)

For lists where items are correlated by an `index` field (tasks, completions, tool calls):

1. Iterate through `b` items
2. For each `b` item, find the matching `a` item by `index`
3. If found: call the element's `merged()` — track if it changed
4. If not found: append as new item — mark changed
5. Return original array `a` if nothing changed, new array if anything did

This is the TypeScript equivalent of the Rust pattern:
```rust
for b_item in other.items {
    if let Some(a_item) = self.items.iter_mut().find(|x| x.index == b_item.index) {
        a_item.push(b_item);
    } else {
        self.items.push(b_item);
    }
}
```

### Numeric Array Comparison (Scores, Weights)

For score/weight arrays that get **replaced** (not accumulated), compare element-by-element:
```typescript
function merged(a: number[], b: number[]): [number[], boolean] {
  if (a.length === b.length) {
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return [b, true];
    }
    return [a, false]; // Same content — preserve reference
  }
  return [b, true]; // Different length — definitely changed
}
```

### Types That Need Merge Functions

Every streaming chunk type needs `merged()`. Every list of indexed streaming chunks needs `mergedList()`. The hierarchy mirrors `objectiveai-rs` push methods:

**Agent completions:**
- `AgentCompletionChunk` — merges messages, usage, error
- `AssistantResponseChunk` — merges reasoning, tool_calls, content, refusal, logprobs, finish_reason, usage
- `MessageChunk` — dispatches to AssistantResponseChunk based on variant
- `Logprobs` — extends content/refusal arrays
- `Usage` / `UpstreamUsage` — sums all token/cost fields
- `CompletionTokensDetails` / `PromptTokensDetails` / `CostDetails` — sums sub-fields

**Vector completions:**
- `VectorCompletionChunk` — merges completions, votes, scores, weights, usage
- `AgentCompletionChunk` (vector context) — delegates to agent version

**Function executions:**
- `FunctionExecutionChunk` — merges tasks, reasoning, output, retry_token, error, usage
- `FunctionExecutionTaskChunk` — delegates to FunctionExecutionChunk
- `VectorCompletionTaskChunk` — delegates to VectorCompletionChunk + error
- `TaskChunk` — dispatches by variant (function vs vector)
- `ReasoningSummaryChunk` — delegates to AgentCompletionChunk + error

**Function inventions:**
- `FunctionInventionChunk` — merges completions, state, path, function, usage, error

**Profile computations:**
- `FunctionProfileComputationChunk` — merges executions, profile, fitting_stats, usage

### Key Difference from Rust

Rust `push()` is `&mut self` — it mutates in place. TypeScript `merged()` is pure — it returns a new object (or the original if unchanged). This is intentional: React's rendering model requires immutable updates where identity change signals re-render need.
