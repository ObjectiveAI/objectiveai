# SDK

## Pipeline

```
objectiveai-rs (Rust types + JsonSchema derives)
    ↓  cargo run --package objectiveai-json-schema-builder
objectiveai-json-schema/ (328 .json files, one per type)
    ↓                           ↓
objectiveai-js/                 objectiveai-py/              objectiveai-go/
  scripts/install-zod.cjs         scripts/install_pydantic.py   scripts/install_go.go
  → Zod schemas in src/           → Pydantic v2 models in      → Go structs in objectiveai/
  scripts/install-wasm.cjs          objectiveai/                scripts/install_cffi.go
  → WASM binary embedded         scripts/install_pyo3.py        → CFFI WASM in lib/
                                  → PyO3 wheel installed
```

## JSON Schemas

`objectiveai-json-schema/` contains one `.json` file per public serializable type in `objectiveai-rs`. Names use dot-separated module paths (e.g., `functions.executions.RetryToken.json`).

The builder normalizes schemas (strips `$defs`, rewrites `$ref`, canonical key order) and enforces structural guarantees tested in `objectiveai-json-schema/builder/tests/schema_properties.rs`.

Types gated behind Cargo features (e.g., `config`) are conditionally included in `json_schemas()`.

## TypeScript SDK (`objectiveai-js`)

Published as `objectiveai` on npm.

- **Types:** Auto-generated Zod schemas from JSON schemas. Circular dependencies use `z.lazy()` with explicit `z.ZodType<T>` annotations. Each module gets a `generatedIndex.ts` barrel export.
- **Client:** `src/client.ts` — HTTP client with Zod-validated options, env var fallbacks, streaming support.
- **Merge system:** `src/merge.ts` — Immutable `merge(a, b) → [result, boolean]` for streaming chunk accumulation. Returns original reference when unchanged (React identity optimization).
- **WASM:** Compiled from `objectiveai-rs-wasm-js`, embedded as base64 at build time (no filesystem dependency).
- **Build:** `pnpm run build` via tsup → `dist/` with CJS + ESM.
- **Tests:** vitest + `tsc --noEmit`. HTTP integration tests use shared API server.

## Python SDK (`objectiveai-py`)

- **Types:** Auto-generated Pydantic v2 models from JSON schemas. Each module gets `__init__.py` barrel exports.
- **Client:** `objectiveai/client.py` — httpx-based HTTP client with env var fallbacks, streaming support.
- **Push system:** `objectiveai/push_utils.py` — Mutable push-style streaming chunk accumulation (Python equivalent of merge).
- **PyO3:** Compiled Rust bindings from `objectiveai-rs-pyo3`, distributed as wheel, installed into venv at build time.
- **Build:** venv-based. Never run bare `python`/`pip` — always use `objectiveai-py/venv/Scripts/python.exe`.
- **Tests:** pytest. HTTP integration tests use shared API server.

## Go SDK (`objectiveai-go`)

- **Types:** Auto-generated Go structs from JSON schemas. Named variants from `$title`, strict `UnmarshalJSON` for required field validation, embedded structs for `$ref` inheritance.
- **Client:** `objectiveai/client.go` — HTTP client with env var fallbacks, typed generic `PostUnary[T]`/`PostStreaming[T]`/`GetUnary[T]`/`DeleteUnary[T]`, SSE streaming with context-based cancellation.
- **Push system:** `objectiveai/*_methods.go` — Mutable push-style streaming chunk accumulation (Go equivalent of merge/push).
- **CFFI:** Compiled from `objectiveai-rs-cffi` as `wasm32-wasip1`, executed via wazero. Provides chunk-to-unary conversion, normalization, generation, and merge verification.
- **Build:** `bash objectiveai-go/build.sh` runs `install_go.go` (types) + `install_cffi.go` (WASM).
- **Tests:** `go test`. HTTP integration tests use shared API server.

## Roundtrip Tests

Each SDK has a roundtrip test that converts generated types back to JSON Schema and asserts equality with the originals. The tests are fully generic — no schema-specific logic.

| SDK | Harness | Test |
|-----|---------|------|
| TypeScript | `src/tests/test-zod-roundtrip-harness.ts` | `src/tests/test-zod-roundtrip.test.ts` |
| Python | `tests/test_pydantic_roundtrip_harness.py` | `tests/test_pydantic_roundtrip.py` |
| Go | `tests/roundtrip_harness_test.go` | `tests/roundtrip_test.go` |

## Build & Test

```bash
bash build.sh   # 3-phase parallel build (schemas → WASM/PyO3 → JS/Python)
bash test.sh     # spawns API server, runs all suites in parallel
```

Individual suites: `bash objectiveai-{js,py,rs,go}/test.sh`
