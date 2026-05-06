# Function Execution Error Paths

Every unique error path that can occur during a function execution, including delegated calls (flat task profile fetching, vector completions, etc.). Each entry includes the trigger condition and a test strategy.

---

## 1. Pre-Execution Errors (early `Err(...)` return from `create_streaming`)

These terminate the stream before any chunks are produced.

### 1.1 `InvalidRetryToken` (400)
**Trigger:** `retry_token` field contains a string that fails `RetryToken::try_from_string()`.
**Location:** `client.rs:682-688`
**Strategy:** Pass a garbage `retry_token` string in the request body. Does NOT require any mock function/profile — just an invalid token string. However, current test setup always uses `retry_token: None`. Would need to add a test that sets `retry_token: Some("invalid")`.

### 1.2 `InvalidFunctionForStrategy` — missing input_split/input_merge (400)
**Trigger:** Request has `strategy: SwissSystem` but the inline function is not a vector type with both `input_split` and `input_merge`.
**Location:** `client.rs:705-717`
**Strategy:** UNABLE TO TRIGGER with mock remote functions. This check only fires for `request.inline_function()` which returns `Some(...)` only for `Request::FunctionInlineProfileRemote` or `Request::FunctionInlineProfileInline` variants. Our tests use `Request::FunctionRemoteProfileRemote`. Would need a new request variant with an inline scalar function + Swiss strategy.

### 1.3 `InvalidFunctionForStrategy` — scalar function (400)
**Trigger:** Request has `strategy: SwissSystem` but the fetched function transpiles to `FunctionType::Scalar`.
**Location:** `client.rs:731-743`
**Strategy:** Create a mock scalar function (e.g. mock-1) and a request with `strategy: Some(SwissSystem { pool: None, rounds: None })`. The FTP fetch will succeed but the type check will fail.

### 1.4 `InvalidStrategy` — invalid pool/rounds (400)
**Trigger:** Swiss system with `pool <= 1` or `rounds == 0`.
**Location:** `client.rs:870-874`
**Strategy:** Create a mock vector function and request with `strategy: Some(SwissSystem { pool: Some(1), rounds: Some(1) })`. Requires a vector function with `input_split` and `input_merge` (remote functions always have these). The pool check at 870 fires before any execution.

---

## 2. Flat Task Profile Fetch Errors (`get_flat_task_profile`)

These also terminate early via `?` propagation.

### 2.1 `FunctionNotFound` (404)
**Trigger:** Function fetcher returns `Ok(None)` — repository not found.
**Location:** `flat_task_profile.rs:509-510`
**Strategy:** Use a non-existent mock repository name like `"mock-nonexistent"`. The mock fetcher's match falls through to `_ => return Ok(None)`.

### 2.2 `FetchFunction` (varies)
**Trigger:** Function fetcher returns `Err(...)` — fetch itself failed.
**Location:** `flat_task_profile.rs:511-512`
**Strategy:** UNABLE TO TRIGGER with `MockFetcher` alone — it only returns `Ok(None)` or `Ok(Some(...))`. Would need a custom fetcher that returns `Err(...)`, or a mock function whose JSON is invalid (panics at `expect("invalid mock function JSON")`). Actually, a malformed JSON in a mock fixture would panic, not error. Truly untriggerable with the current mock fetcher.

### 2.3 `ProfileNotFound` (404)
**Trigger:** Profile fetcher returns `Ok(None)`.
**Location:** `flat_task_profile.rs:526-527`
**Strategy:** Use a valid mock function but a non-existent profile repository like `"mock-nonexistent"`.

### 2.4 `FetchProfile` (varies)
**Trigger:** Profile fetcher returns `Err(...)`.
**Location:** `flat_task_profile.rs:527-528`
**Strategy:** UNABLE TO TRIGGER with `MockFetcher` — same as 2.2.

### 2.5 `InputSchemaMismatch` (400)
**Trigger:** Function has an `input_schema` and `input_schema.validate_input(&input)` returns false.
**Location:** `flat_task_profile.rs:606-607`
**Strategy:** Use a mock function (e.g. mock-1 expects `{"text": string}`) but pass an input that doesn't match, like `Input::String("wrong")` or `Input::Object({"wrong_field": ...})`.

### 2.6 `InvalidProfile` — tasks length mismatch (400)
**Trigger:** Tasks-based profile's `tasks` array length != function's tasks count.
**Location:** `flat_task_profile.rs:627-631` (RemoteTasksProfile), `649-653` (InlineTasksProfile)
**Strategy:** Create a mock function with 2 tasks but a tasks-based profile with 3 task entries (or vice versa). E.g., pair mock-9 (2 tasks) with a profile that has `"tasks": [{...}, {...}, {...}]`.

### 2.7 `InvalidProfile` — weights length mismatch (400)
**Trigger:** Tasks-based profile's `profile` weights count != function's tasks count.
**Location:** `flat_task_profile.rs:634-638` (RemoteTasksProfile), `656-660` (InlineTasksProfile)
**Strategy:** Create a mock profile with `"tasks": [{...}, {...}]` matching function tasks count but `"profile": [0.5, 0.3, 0.2]` (3 weights for 2 tasks).

### 2.8 `InvalidProfile` — wrong TaskProfile type for function task (400)
**Trigger:** A function task is a sub-function (ScalarFunction/VectorFunction) but the corresponding TaskProfile is `Placeholder {}` instead of `Remote` or `Inline`.
**Location:** `flat_task_profile.rs:830-832`
**Strategy:** Create a tasks-based profile where a branch sub-function task gets `{}` (placeholder) instead of a remote reference or inline profile.

### 2.9 `InvalidProfile` — wrong TaskProfile type for VC task (400)
**Trigger:** A function task is a VectorCompletion but the corresponding TaskProfile is not `Inline(Auto)`.
**Location:** `flat_task_profile.rs:876-878`
**Strategy:** Create a tasks-based profile for a leaf function where a VC task gets a `Remote` reference or `Inline(Tasks)` instead of `Inline(Auto)` with swarm+profile.

### 2.10 `InvalidProfile` — wrong TaskProfile type for placeholder scalar (400)
**Trigger:** A PlaceholderScalarFunction task has a non-Placeholder TaskProfile.
**Location:** `flat_task_profile.rs:903-905`
**Strategy:** UNABLE TO TRIGGER with alpha functions — alpha functions transpile scalar/vector function tasks and VC tasks, not placeholder tasks. Placeholder tasks come from standard `RemoteFunction::Scalar`/`Vector` definitions, not alpha functions. Would require a non-alpha function fixture.

### 2.11 `InvalidProfile` — wrong TaskProfile type for placeholder vector (400)
**Trigger:** A PlaceholderVectorFunction task has a non-Placeholder TaskProfile.
**Location:** `flat_task_profile.rs:926-928`
**Strategy:** Same as 2.10 — UNABLE TO TRIGGER with alpha functions.

### 2.12 `InvalidProfile` — wrong TaskProfile for mapped VC task (400)
**Trigger:** A mapped VectorCompletion task has a TaskProfile that is not `Inline(Auto)`.
**Location:** `flat_task_profile.rs:1002-1004`
**Strategy:** UNABLE TO TRIGGER with current alpha functions — mapped tasks require `input_maps` which alpha functions don't support. Would need a standard `RemoteFunction` with `input_maps`.

### 2.13 `InvalidProfile` — wrong TaskProfile for mapped function task (400)
**Trigger:** A mapped sub-function task has a `Placeholder {}` TaskProfile.
**Location:** `flat_task_profile.rs:1099-1101`
**Strategy:** Same as 2.12 — requires mapped function tasks.

### 2.14 `InvalidProfile` — wrong TaskProfile for mapped placeholder scalar (400)
**Location:** `flat_task_profile.rs:1145-1147`
**Strategy:** UNABLE TO TRIGGER — requires mapped placeholder tasks.

### 2.15 `InvalidProfile` — wrong TaskProfile for mapped placeholder vector (400)
**Location:** `flat_task_profile.rs:1180-1182`
**Strategy:** UNABLE TO TRIGGER — requires mapped placeholder tasks.

### 2.16 `InvalidAppExpression` — output_length compile fails (400)
**Trigger:** A `RemoteFunction::Vector`'s `output_length` expression fails to evaluate.
**Location:** `flat_task_profile.rs:717` (function level), `flat_task_profile.rs:940` (placeholder vector task level), `flat_task_profile.rs:1201` (mapped placeholder vector)
**Strategy:** For 717: Create a mock vector function whose `output_length` expression references a missing input field. E.g. `{"$starlark": "len(input['nonexistent'])"}` with input that lacks that field.

### 2.17 `InvalidAppExpression` — compile_tasks fails (400)
**Trigger:** Any expression in the function's tasks fails to compile — skip expression, map expression, input expression, or task field expression.
**Location:** `flat_task_profile.rs:740` (via `function.compile_tasks(&input)?`)
**Strategy:** Create a mock function with a Starlark expression that references a missing key, e.g. a task input expression like `{"$starlark": "input['missing_key']"}` when the input doesn't have that key.

### 2.18 `SwarmNotFound` (404)
**Trigger:** A VC task's swarm is specified by ID string and that ID is not found.
**Location:** `flat_task_profile.rs:1271`
**Strategy:** Create a mock profile with an inline auto profile whose `swarm` is a string ID (e.g. `"swarm": "nonexistent_id"`). The `StubSwarmFetcher` in tests returns `Err(501)`, so this would actually trigger `FetchSwarm` instead. To get `SwarmNotFound`, would need an swarm fetcher that returns `Ok(None)`.

### 2.19 `FetchSwarm` (varies)
**Trigger:** Swarm fetcher returns `Err(...)` when looking up an swarm ID.
**Location:** `flat_task_profile.rs:1272`
**Strategy:** Same as 2.18 — the `StubSwarmFetcher` returns `Err(ResponseError { code: 501, ... })`. So using a string swarm ID in a mock profile will trigger this. Create a profile with `"swarm": "some-id"` instead of `"swarm": {"agents": [...]}`.

### 2.20 `InvalidSwarm` (400)
**Trigger:** `Swarm::try_from_with_profile()` fails — e.g. profile length doesn't match swarm agent count after deduplication/merging.
**Location:** `flat_task_profile.rs:1285`
**Strategy:** Create a mock profile where an inline auto VC profile has `"swarm": {"agents": [{"upstream": "mock", "output_mode": "json_schema"}]}` with `"profile": [0.5, 0.5]` — 1 agent but 2 weights.

### 2.21 Recursive `FunctionNotFound` in sub-function task (404)
**Trigger:** A branch function references a sub-function that doesn't exist (e.g. `"repository": "mock-nonexistent"`).
**Location:** `flat_task_profile.rs:509-510` (recursed from 849)
**Strategy:** Create a mock branch function whose task references `"repository": "mock-999"` which doesn't exist in the mock fetcher. The recursive `get_flat_task_profile` call will return `FunctionNotFound`.

### 2.22 Recursive `ProfileNotFound` in sub-function task (404)
**Trigger:** A tasks-based profile references a remote sub-profile that doesn't exist.
**Location:** `flat_task_profile.rs:526-527` (recursed)
**Strategy:** Create a tasks-based mock profile with a `TaskProfile::Remote` pointing to `"repository": "mock-999"`. The recursive profile fetch will return `ProfileNotFound`.

### 2.23 Recursive `InputSchemaMismatch` in sub-function (400)
**Trigger:** A branch function passes input to a sub-function that doesn't match the sub-function's schema.
**Location:** `flat_task_profile.rs:606-607` (recursed)
**Strategy:** Create a mock branch function whose input expression for a task produces output that doesn't match the sub-function's input_schema. E.g. a task input expression `{"$starlark": "{'wrong': input['text']}"}` when the sub-function expects `{"text": ...}`.

---

## 3. Vector Completion Errors (during execution)

These are caught gracefully — they produce error chunks but don't terminate the stream.

### 3.1 `Vector(...)` — VC create_streaming fails (varies)
**Trigger:** The vector completions client fails to create a stream for a VC task. This wraps `vector::completions::Error`.
**Location:** `client.rs:~2530-2586` (caught in the VC execution arm)
**Strategy:** The mock agent client returns `Err(super::Error::ExpectedError)` when `agent.base.error == Some(true)`. So a profile with `"error": true` on ALL agents (no fallbacks) for a VC task will cause the vector completion to fail. Mock-10's profile already has one error agent, but it has non-error agents too. Create a profile where ALL agents have `"error": true`.

### 3.2 Agent error during VC streaming (varies)
**Trigger:** An agent produces an error chunk during streaming (after initial success).
**Location:** Handled inside `vector/completions/client.rs` — produces an error field on the VectorCompletionChunk.
**Strategy:** This is an upstream-level error. The mock client either succeeds or fails upfront (no mid-stream errors). UNABLE TO TRIGGER with mock client.

---

## 4. Task Output Expression Errors (during execution)

These are accumulated and reported in the final chunk.

### 4.1 `InvalidAppExpression` — output expression evaluation fails (400)
**Trigger:** A task's output expression fails to evaluate given the task's actual output.
**Location:** `client.rs:203-213` (via `apply_task_output_expression`)
**Strategy:** Create a mock function whose output expression references a field that doesn't exist on the output. E.g. `{"$starlark": "output['nonexistent']"}`. The output is a `TaskOutputOwned` (for function tasks) or VC scores (for VC tasks), and the expression must fail on the actual mock output.

### 4.2 `InvalidScalarOutput` — scalar out of range (400)
**Trigger:** Scalar function's output expression produces a value outside [-0.01, 1.01].
**Location:** `client.rs:220-229`
**Strategy:** Create a scalar function with an output expression like `{"$starlark": "output['scores'][0] * 10"}` that amplifies a score beyond 1.0. Or `{"$starlark": "-1.0"}`.

### 4.3 `InvalidScalarOutput` — scalar function got vector (400)
**Trigger:** Scalar function's output expression returns a vector instead of a scalar.
**Location:** `client.rs:232-240`
**Strategy:** Create a scalar function with an output expression like `{"$starlark": "output['scores']"}` (returns full scores array instead of a single element).

### 4.4 `InvalidVectorOutput` — sum not ~1 or length mismatch (400)
**Trigger:** Vector function's output expression returns a vector whose sum is outside [0.99, 1.01] or whose length doesn't match `output_length`.
**Location:** `client.rs:246-260`
**Strategy:** Create a vector function with an output expression that returns scores multiplied by 2, or returns a vector of the wrong length.

### 4.5 `InvalidVectorOutput` — vector function got scalar (400)
**Trigger:** Vector function's output expression returns a scalar instead of a vector.
**Location:** `client.rs:263-272`
**Strategy:** Create a vector function with an output expression like `{"$starlark": "output['scores'][0]"}` (extracts one element instead of returning the vector).

### 4.6 `InvalidScalarOutput` — unexpected Vectors variant (400)
**Trigger:** Output expression produces `TaskOutputOwned::Vectors` (nested vectors).
**Location:** `client.rs:275-280`
**Strategy:** Unclear how to trigger — `Vectors` comes from mapped tasks but `apply_task_output_expression` is called per-task. The expression would need to construct a nested list from a non-mapped context. Possibly `{"$starlark": "[[0.5, 0.5]]"}` — a list containing a list.

### 4.7 Task output expression error — expression returns Err value (400)
**Trigger:** Output expression evaluates successfully but produces `TaskOutputOwned::Err { error: ... }`.
**Location:** `client.rs:282-291`
**Strategy:** Create an output expression that returns `None` in Starlark (maps to `TaskOutputOwned::Err { error: null }`). E.g. `{"$starlark": "None"}`.

### 4.8 `TaskOutputExpressionErrors` aggregation (400)
**Trigger:** One or more tasks produced output expression errors (any of 4.1-4.7). These are collected in `task_output_errors` and reported together.
**Location:** `client.rs:2381-2387`
**Strategy:** Same as any of 4.1-4.7 — the error is accumulated and set on the final chunk's `error` field.

### 4.9 Empty map task output expression error (400)
**Trigger:** A mapped task produces zero instances (empty map) and the output expression fails on the empty result.
**Location:** `client.rs:2168-2172`
**Strategy:** UNABLE TO TRIGGER with current alpha functions — requires mapped tasks.

---

## 5. Swiss System Subsequent Round Errors

These are non-fatal — execution completes and the error is reported in the final chunk.

### 5.1 `InvalidAppExpression` — input_merge fails in subsequent round (400)
**Trigger:** `input_merge.compile_one()` fails when re-merging items after re-sorting.
**Location:** `client.rs:~1204-1224`
**Strategy:** UNLIKELY TO TRIGGER — if input_merge works for round 1, it should work for subsequent rounds since the data shape doesn't change.

### 5.2 FTP fetch fails in subsequent round (varies)
**Trigger:** `fetch_function_flat_task_profile` fails for a re-pooled chunk in round >1.
**Location:** `client.rs:~1232-1240`
**Strategy:** UNLIKELY TO TRIGGER with mocks — if the function/profile exist for round 1, they'll exist for subsequent rounds too.

---

## 6. Reasoning Errors

### 6.1 Agent completions error during reasoning (varies)
**Trigger:** The reasoning model (agent completion) fails.
**Location:** `client.rs:~2738-2778`
**Strategy:** Pass `reasoning: Some(Reasoning { agent: Agent::Provided(mock_agent_base_with_error_true), agents: None })` in the request. The mock agent with `error: true` will fail, producing an error chunk in the reasoning stream. Fully triggerable with mock agents.

---

## 7. `NoValidTaskOutputs` (400)

**Trigger:** Defined in `error.rs` but NEVER GENERATED in `client.rs`. The `compute_weighted_function_output` function handles this case by returning `TaskOutputOwned::Err { error: null }` instead, which becomes the function's output. This error variant appears to be dead code.
**Strategy:** N/A — cannot be triggered.

---

## Summary: Triggerable via Mock

| # | Error | Triggerable? | Notes |
|---|-------|-------------|-------|
| 1.1 | InvalidRetryToken | Yes | Invalid retry_token string |
| 1.2 | InvalidFunctionForStrategy (inline) | No | Requires inline function request variant |
| 1.3 | InvalidFunctionForStrategy (scalar) | Yes | Scalar function + Swiss strategy |
| 1.4 | InvalidStrategy | Yes | pool=1 or rounds=0 with Swiss |
| 2.1 | FunctionNotFound | Yes | Non-existent mock repo |
| 2.2 | FetchFunction | No | MockFetcher can't return Err |
| 2.3 | ProfileNotFound | Yes | Non-existent mock profile repo |
| 2.4 | FetchProfile | No | MockFetcher can't return Err |
| 2.5 | InputSchemaMismatch | Yes | Wrong input shape |
| 2.6 | InvalidProfile (tasks len) | Yes | Mismatched tasks array length |
| 2.7 | InvalidProfile (weights len) | Yes | Mismatched weights array length |
| 2.8 | InvalidProfile (func task) | Yes | Placeholder for function task |
| 2.9 | InvalidProfile (VC task) | Yes | Non-Auto for VC task |
| 2.10 | InvalidProfile (placeholder scalar) | No | Requires non-alpha function |
| 2.11 | InvalidProfile (placeholder vector) | No | Requires non-alpha function |
| 2.12 | InvalidProfile (mapped VC) | No | Requires input_maps |
| 2.13 | InvalidProfile (mapped func) | No | Requires input_maps |
| 2.14 | InvalidProfile (mapped placeholder scalar) | No | Requires input_maps |
| 2.15 | InvalidProfile (mapped placeholder vector) | No | Requires input_maps |
| 2.16 | InvalidAppExpression (output_length) | Yes | Bad output_length expression |
| 2.17 | InvalidAppExpression (compile_tasks) | Yes | Bad task expression |
| 2.18 | SwarmNotFound | No | StubSwarmFetcher returns Err, not Ok(None) |
| 2.19 | FetchSwarm | Yes | String swarm ID hits StubSwarmFetcher |
| 2.20 | InvalidSwarm | Yes | Profile/swarm agent count mismatch |
| 2.21 | Recursive FunctionNotFound | Yes | Sub-function repo doesn't exist |
| 2.22 | Recursive ProfileNotFound | Yes | Sub-profile repo doesn't exist |
| 2.23 | Recursive InputSchemaMismatch | Yes | Bad input expression for sub-function |
| 3.1 | Vector (VC fails) | Yes | All agents have error=true |
| 3.2 | Agent mid-stream error | No | Mock client doesn't error mid-stream |
| 4.1 | InvalidAppExpression (output expr) | Yes | Bad output expression |
| 4.2 | InvalidScalarOutput (range) | Yes | Output expression amplifies score |
| 4.3 | InvalidScalarOutput (got vector) | Yes | Output returns array for scalar func |
| 4.4 | InvalidVectorOutput (sum/len) | Yes | Output expression distorts scores |
| 4.5 | InvalidVectorOutput (got scalar) | Yes | Output returns scalar for vector func |
| 4.6 | InvalidScalarOutput (Vectors) | Maybe | Expression returns nested list |
| 4.7 | Task output Err value | Yes | Expression returns None |
| 4.8 | TaskOutputExpressionErrors | Yes | Any of 4.1-4.7 |
| 4.9 | Empty map output expr | No | Requires mapped tasks |
| 5.1 | Swiss input_merge (round >1) | No | Unlikely + requires Swiss strategy |
| 5.2 | Swiss FTP fetch (round >1) | No | Unlikely + requires Swiss strategy |
| 6.1 | Reasoning agent error | Yes | Mock agent with error=true as reasoning agent |
| 7 | NoValidTaskOutputs | No | Dead code — never generated |
