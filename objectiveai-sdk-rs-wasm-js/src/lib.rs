//! WebAssembly bindings for ObjectiveAI.
//!
//! This crate provides JavaScript/TypeScript bindings for client-side validation
//! and compilation of ObjectiveAI types. It enables browser-based applications to:
//!
//! - Validate Swarm LLM and Swarm configurations
//! - Compute content-addressed IDs (deterministic hashes)
//! - Compile Function expressions for previewing during authoring
//! - Compute prompt, tools, and response IDs for caching/deduplication
//!
//! # Usage
//!
//! This crate is compiled to WebAssembly and consumed via the `objectiveai` npm package.
//! The TypeScript SDK wraps these functions with proper type definitions.
//!
//! # Functions
//!
//! - [`validateAgent`] - Validate and compute ID for an Agent
//! - [`validateSwarm`] - Validate and compute ID for an Swarm
//! - [`compileFunctionTasks`] - Compile function tasks for a given input
//! - [`compileFunctionOutput`] - Compile function output from task results
//! - [`promptId`] - Compute content-addressed ID for chat messages
//! - [`vectorResponseId`] - Compute content-addressed ID for a response option

#![allow(non_snake_case)]
use arbitrary::Arbitrary;
use wasm_bindgen::prelude::*;

fn seed_to_bytes(seed: JsValue) -> Vec<u8> {
    use rand::prelude::*;
    use std::hash::{Hash, Hasher};

    let seed_val: u64 = if seed.is_undefined() || seed.is_null() {
        rand::random()
    } else {
        // Hash any JsValue into a u64 seed via its debug representation
        let repr = format!("{:?}", seed);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        repr.hash(&mut hasher);
        hasher.finish()
    };

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed_val);
    let mut bytes = vec![0u8; 4096];
    rng.fill_bytes(&mut bytes);
    bytes
}

/// Validates an Agent configuration and computes its content-addressed ID.
///
/// Takes an Agent definition, normalizes it (removes defaults, deduplicates),
/// validates all fields, and computes a deterministic ID using XXHash3-128.
///
/// # Arguments
///
/// * `agent` - JavaScript object representing an Agent configuration
///
/// # Returns
///
/// The validated Agent with its computed `id` field populated.
///
/// # Errors
///
/// Returns an error string if validation fails (e.g., invalid model name,
/// out-of-range parameters, conflicting settings).
#[wasm_bindgen]
pub fn validateAgent(agent: JsValue) -> Result<String, JsValue> {
    // deserialize
    let agent_base: objectiveai_sdk::agent::AgentBase =
        serde_wasm_bindgen::from_value(agent)?;
    // prepare, validate, and compute ID
    let agent: objectiveai_sdk::agent::Agent = agent_base
        .convert()
        .map_err(|e| JsValue::from_str(&e))?;
    // serialize
    serde_json::to_string(&agent).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validates an Swarm configuration and computes its content-addressed ID.
///
/// Takes an Swarm definition (a collection of Swarm LLMs), validates each
/// LLM, and computes a deterministic ID for the swarm as a whole.
///
/// # Arguments
///
/// * `swarm` - JavaScript object representing an Swarm configuration
///
/// # Returns
///
/// The validated Swarm with its computed `id` field populated and all
/// member LLMs validated with their IDs.
///
/// # Errors
///
/// Returns an error string if any LLM validation fails or the swarm
/// structure is invalid.
#[wasm_bindgen]
pub fn validateSwarm(swarm: JsValue, remote_agents: JsValue) -> Result<String, JsValue> {
    // deserialize
    let swarm_base: objectiveai_sdk::swarm::SwarmBase =
        serde_wasm_bindgen::from_value(swarm)?;
    // Values are `(base, path)` tuples to match `SwarmBase::convert`'s
    // signature; the RemotePath half is ignored by the conversion (it
    // resolves via `.0`).
    let remote_agents: Option<std::collections::HashMap<String, (objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks, objectiveai_sdk::RemotePath)>> =
        if remote_agents.is_undefined() || remote_agents.is_null() {
            None
        } else {
            Some(serde_wasm_bindgen::from_value(remote_agents)?)
        };
    // prepare, validate, and compute ID
    let swarm: objectiveai_sdk::swarm::Swarm = swarm_base
        .convert(remote_agents.as_ref())
        .map_err(|e| JsValue::from_str(&e))?;
    // serialize
    serde_json::to_string(&swarm).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validates function input against its schema.
///
/// For remote functions, checks whether the provided input conforms to
/// the function's JSON Schema definition. For inline functions, returns
/// `null` since they lack schema definitions.
///
/// # Arguments
///
/// * `function` - JavaScript object representing a Function definition
/// * `input` - JavaScript object representing the function input to validate
///
/// # Returns
///
/// - `true` if the input is valid against the schema
/// - `false` if the input is invalid
/// - `null` for inline functions (no schema to validate against)
///
/// # Errors
///
/// Returns an error if deserialization fails.
#[wasm_bindgen]
pub fn validateFunctionInput(
    function: JsValue,
    input: JsValue,
) -> Result<Option<bool>, JsValue> {
    // deserialize
    let function: objectiveai_sdk::functions::Function =
        serde_wasm_bindgen::from_value(function)?;
    let input: objectiveai_sdk::functions::expression::InputValue =
        serde_wasm_bindgen::from_value(input)?;
    // validate input
    Ok(function.validate_input(&input))
}

/// Compiles a Function's task expressions for a given input.
///
/// Evaluates all expressions (JMESPath or Starlark) in the function's tasks
/// using the provided input data. This is used for previewing how tasks will
/// be executed during Function authoring.
///
/// # Arguments
///
/// * `function` - JavaScript object representing a Function definition
/// * `input` - JavaScript object representing the function input
///
/// # Returns
///
/// An array where each element corresponds to a task definition:
/// - `null` if the task was skipped (skip expression evaluated to true)
/// - `{ One: task }` for non-mapped tasks
/// - `{ Many: [task, ...] }` for mapped tasks (expanded from map expression)
///
/// # Errors
///
/// Returns an error string if expression evaluation fails or types don't match.
#[wasm_bindgen]
pub fn compileFunctionTasks(
    function: JsValue,
    input: JsValue,
) -> Result<String, JsValue> {
    // deserialize
    let function: objectiveai_sdk::functions::Function =
        serde_wasm_bindgen::from_value(function)?;
    let input: objectiveai_sdk::functions::expression::InputValue =
        serde_wasm_bindgen::from_value(input)?;
    // compile tasks
    let tasks = function
        .compile_tasks(&input)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    // serialize
    serde_json::to_string(&tasks).map_err(|e| JsValue::from_str(&e.to_string()))
}

// TODO: Update for new per-task output expression architecture
// /// Computes the final output of a Function given input and task results.
// ///
// /// Evaluates the function's output expression using the provided input data
// /// and task outputs. Also validates that the output meets constraints:
// /// - Scalar functions: output must be in [0, 1]
// /// - Vector functions: output must sum to approximately 1
// ///
// /// # Arguments
// ///
// /// * `function` - JavaScript object representing a Function definition
// /// * `input` - JavaScript object representing the function input
// /// * `task_outputs` - Array of task outputs (from actual execution or mocked)
// ///
// /// # Returns
// ///
// /// An object with:
// /// - `output`: The computed scalar or vector output
// /// - `valid`: Boolean indicating if the output meets constraints
// ///
// /// # Errors
// ///
// /// Returns an error string if expression evaluation fails.
// #[wasm_bindgen]
// pub fn compileFunctionOutput(
//     function: JsValue,
//     input: JsValue,
//     task_outputs: JsValue,
// ) -> Result<JsValue, JsValue> {
//     // deserialize
//     let function: objectiveai_sdk::functions::Function =
//         serde_wasm_bindgen::from_value(function)?;
//     let input: objectiveai_sdk::functions::expression::InputValue =
//         serde_wasm_bindgen::from_value(input)?;
//     let task_outputs: Vec<
//         Option<objectiveai_sdk::functions::expression::TaskOutput<'static>>,
//     > = serde_wasm_bindgen::from_value(task_outputs)?;
//     // compile output
//     let output = function
//         .compile_output(&input, &task_outputs)
//         .map_err(|e| JsValue::from_str(&e.to_string()))?;
//     // serialize
//     let output: JsValue = serde_wasm_bindgen::to_value(&output)?;
//     Ok(output)
// }

/// Computes the expected output length for a vector Function.
///
/// Evaluates the `output_length` expression to determine how many elements
/// the output vector should contain. This is only applicable to remote
/// vector functions which have an `output_length` field.
///
/// # Arguments
///
/// * `function` - JavaScript object representing a Function definition
/// * `input` - JavaScript object representing the function input
///
/// # Returns
///
/// - The expected output length for remote vector functions
/// - `null` for scalar functions or inline functions
///
/// # Errors
///
/// Returns an error string if expression evaluation fails.
#[wasm_bindgen]
pub fn compileFunctionOutputLength(
    function: JsValue,
    input: JsValue,
) -> Result<Option<u32>, JsValue> {
    // deserialize
    let function: objectiveai_sdk::functions::Function =
        serde_wasm_bindgen::from_value(function)?;
    let input: objectiveai_sdk::functions::expression::InputValue =
        serde_wasm_bindgen::from_value(input)?;
    // compile output length
    Ok(function
        .compile_output_length(&input)
        .map_err(|e| JsValue::from_str(&e.to_string()))?
        .map(|u| u as u32))
}

/// Compiles the `input_split` expression to split input into multiple sub-inputs.
///
/// Used by strategies like Swiss System that need to partition input into
/// smaller pools. The expression transforms the original input into an array
/// of inputs, where each element can be processed independently.
///
/// # Arguments
///
/// * `function` - JavaScript object representing a Function definition
/// * `input` - JavaScript object representing the function input to split
///
/// # Returns
///
/// - An array of split inputs for vector functions with `input_split` defined
/// - `null` for scalar functions or functions without `input_split`
///
/// # Errors
///
/// Returns an error string if expression evaluation fails.
#[wasm_bindgen]
pub fn compileFunctionInputSplit(
    function: JsValue,
    input: JsValue,
) -> Result<Option<String>, JsValue> {
    // deserialize
    let function: objectiveai_sdk::functions::Function =
        serde_wasm_bindgen::from_value(function)?;
    let input: objectiveai_sdk::functions::expression::InputValue =
        serde_wasm_bindgen::from_value(input)?;
    // compile input split
    let input_split = function
        .compile_input_split(&input)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    // serialize
    input_split
        .map(|split| serde_json::to_string(&split).map_err(|e| JsValue::from_str(&e.to_string())))
        .transpose()
}

/// Compiles the `input_merge` expression to merge multiple sub-inputs back into one.
///
/// Used by strategies like Swiss System to recombine a subset of split inputs
/// into a single input for pool execution. The expression transforms an array
/// of inputs (a subset from `compileFunctionInputSplit`) into a single merged input.
///
/// # Arguments
///
/// * `function` - JavaScript object representing a Function definition
/// * `input` - Array of inputs to merge (typically a subset from `compileFunctionInputSplit`)
///
/// # Returns
///
/// - The merged input for vector functions with `input_merge` defined
/// - `null` for scalar functions or functions without `input_merge`
///
/// # Errors
///
/// Returns an error string if expression evaluation fails.
#[wasm_bindgen]
pub fn compileFunctionInputMerge(
    function: JsValue,
    input: JsValue,
) -> Result<Option<String>, JsValue> {
    // deserialize
    let function: objectiveai_sdk::functions::Function =
        serde_wasm_bindgen::from_value(function)?;
    let input: Vec<objectiveai_sdk::functions::expression::InputValue> =
        serde_wasm_bindgen::from_value(input)?;
    // compile input merge
    let input_merge = function
        .compile_input_merge(&objectiveai_sdk::functions::expression::InputValue::Array(
            input,
        ))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    // serialize
    input_merge
        .map(|merge| serde_json::to_string(&merge).map_err(|e| JsValue::from_str(&e.to_string())))
        .transpose()
}

/// Validates vector function fields (output_length, input_split, input_merge).
///
/// Generates diverse example inputs from the input_schema and validates that the
/// output_length, input_split, and input_merge expressions work correctly together
/// via round-trip testing.
#[wasm_bindgen]
pub fn checkVectorFields(fields: JsValue) -> Result<(), JsValue> {
    let fields: objectiveai_sdk::functions::check::VectorFieldsValidation =
        serde_wasm_bindgen::from_value(fields)?;
    objectiveai_sdk::functions::check::check_vector_fields(fields, None)
        .map_err(|e| JsValue::from_str(&e))
}

/// Validates scalar function fields (input_schema only).
#[wasm_bindgen]
pub fn checkScalarFields(fields: JsValue) -> Result<(), JsValue> {
    let fields: objectiveai_sdk::functions::check::ScalarFieldsValidation =
        serde_wasm_bindgen::from_value(fields)?;
    objectiveai_sdk::functions::check::check_scalar_fields(fields, None)
        .map_err(|e| JsValue::from_str(&e))
}

/// Alpha check for a leaf scalar function (depth 0, scalar output).
#[wasm_bindgen]
pub fn alphaCheckLeafScalarFunction(function: JsValue) -> Result<(), JsValue> {
    let function: objectiveai_sdk::functions::alpha_scalar::RemoteFunction =
        serde_wasm_bindgen::from_value(function)?;
    objectiveai_sdk::functions::alpha_scalar::check::check_alpha_leaf_scalar_function(&function, None)
        .map_err(|e| JsValue::from_str(&e))
}

/// Alpha check for a branch scalar function (depth > 0, scalar output).
///
/// `children` is an optional map of child function name → FullRemoteFunction for
/// validating placeholder task inputs against child function input schemas.
#[wasm_bindgen]
pub fn alphaCheckBranchScalarFunction(function: JsValue, children: JsValue) -> Result<(), JsValue> {
    let function: objectiveai_sdk::functions::alpha_scalar::RemoteFunction =
        serde_wasm_bindgen::from_value(function)?;
    let children: Option<std::collections::HashMap<String, objectiveai_sdk::functions::FullRemoteFunction>> =
        if children.is_undefined() || children.is_null() {
            None
        } else {
            Some(serde_wasm_bindgen::from_value(children)?)
        };
    objectiveai_sdk::functions::alpha_scalar::check::check_alpha_branch_scalar_function(&function, children.as_ref(), None)
        .map_err(|e| JsValue::from_str(&e))
}

/// Alpha check for a leaf vector function (depth 0, vector output).
#[wasm_bindgen]
pub fn alphaCheckLeafVectorFunction(function: JsValue) -> Result<(), JsValue> {
    let function: objectiveai_sdk::functions::alpha_vector::RemoteFunction =
        serde_wasm_bindgen::from_value(function)?;
    objectiveai_sdk::functions::alpha_vector::check::check_alpha_leaf_vector_function(&function, None)
        .map_err(|e| JsValue::from_str(&e))
}

/// Alpha check for a branch vector function (depth > 0, vector output).
///
/// `children` is an optional map of child function name → FullRemoteFunction for
/// validating placeholder task inputs against child function input schemas.
#[wasm_bindgen]
pub fn alphaCheckBranchVectorFunction(function: JsValue, children: JsValue) -> Result<(), JsValue> {
    let function: objectiveai_sdk::functions::alpha_vector::RemoteFunction =
        serde_wasm_bindgen::from_value(function)?;
    let children: Option<std::collections::HashMap<String, objectiveai_sdk::functions::FullRemoteFunction>> =
        if children.is_undefined() || children.is_null() {
            None
        } else {
            Some(serde_wasm_bindgen::from_value(children)?)
        };
    objectiveai_sdk::functions::alpha_vector::check::check_alpha_branch_vector_function(&function, children.as_ref(), None)
        .map_err(|e| JsValue::from_str(&e))
}

/// Computes a content-addressed ID for chat messages.
///
/// Normalizes the messages (consolidates text parts, removes empty content)
/// and computes a deterministic hash. This ID is used for caching and
/// deduplicating requests with identical prompts.
///
/// # Arguments
///
/// * `prompt` - Array of chat messages
///
/// # Returns
///
/// A base62-encoded hash string uniquely identifying the prompt content.
///
/// # Errors
///
/// Returns an error if the messages cannot be deserialized.
#[wasm_bindgen]
pub fn promptId(prompt: JsValue) -> Result<String, JsValue> {
    // deserialize
    let mut prompt: Vec<objectiveai_sdk::agent::completions::message::Message> =
        serde_wasm_bindgen::from_value(prompt)?;
    // prepare and compute ID
    objectiveai_sdk::agent::completions::message::prompt::prepare(&mut prompt);
    let id = objectiveai_sdk::agent::completions::message::prompt::id(&prompt);
    Ok(id)
}

/// Computes a content-addressed ID for a vector completion response option.
///
/// Normalizes the response content (consolidates text parts, removes empty
/// content) and computes a deterministic hash. This ID is used for caching
/// and identifying individual response options in vector completions.
///
/// # Arguments
///
/// * `response` - A rich content object (text or multipart content)
///
/// # Returns
///
/// A base62-encoded hash string uniquely identifying the response content.
///
/// # Errors
///
/// Returns an error if the response cannot be deserialized.
#[wasm_bindgen]
pub fn vectorResponseId(response: JsValue) -> Result<String, JsValue> {
    // deserialize
    let mut response: objectiveai_sdk::agent::completions::message::RichContent =
        serde_wasm_bindgen::from_value(response)?;
    // prepare and compute ID
    response.prepare();
    let id = response.id();
    Ok(id)
}

/// Merges two `AgentCompletionChunk`s and returns the merged result.
#[wasm_bindgen]
pub fn agentCompletionChunkMerged(a: JsValue, b: JsValue) -> Result<String, JsValue> {
    let mut a: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
        serde_wasm_bindgen::from_value(a)?;
    let b: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
        serde_wasm_bindgen::from_value(b)?;
    a.push(&b);
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Merges two `VectorCompletionChunk`s and returns the merged result.
#[wasm_bindgen]
pub fn vectorCompletionChunkMerged(a: JsValue, b: JsValue) -> Result<String, JsValue> {
    let mut a: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
        serde_wasm_bindgen::from_value(a)?;
    let b: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
        serde_wasm_bindgen::from_value(b)?;
    a.push(&b);
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Merges two `FunctionExecutionChunk`s and returns the merged result.
#[wasm_bindgen]
pub fn functionExecutionChunkMerged(a: JsValue, b: JsValue) -> Result<String, JsValue> {
    let mut a: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
        serde_wasm_bindgen::from_value(a)?;
    let b: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
        serde_wasm_bindgen::from_value(b)?;
    a.push(&b);
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Merges two `FunctionProfileComputationChunk`s and returns the merged result.
#[wasm_bindgen]
pub fn functionProfileComputationChunkMerged(a: JsValue, b: JsValue) -> Result<String, JsValue> {
    let mut a: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
        serde_wasm_bindgen::from_value(a)?;
    let b: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
        serde_wasm_bindgen::from_value(b)?;
    a.push(&b);
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Normalizes an `AgentCompletionChunk` by round-tripping through serde.
#[wasm_bindgen]
pub fn agentCompletionChunkNormalized(a: JsValue) -> Result<String, JsValue> {
    let a: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
        serde_wasm_bindgen::from_value(a)?;
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Normalizes a `VectorCompletionChunk` by round-tripping through serde.
#[wasm_bindgen]
pub fn vectorCompletionChunkNormalized(a: JsValue) -> Result<String, JsValue> {
    let a: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
        serde_wasm_bindgen::from_value(a)?;
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Normalizes a `FunctionExecutionChunk` by round-tripping through serde.
#[wasm_bindgen]
pub fn functionExecutionChunkNormalized(a: JsValue) -> Result<String, JsValue> {
    let a: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
        serde_wasm_bindgen::from_value(a)?;
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Normalizes a `FunctionProfileComputationChunk` by round-tripping through serde.
#[wasm_bindgen]
pub fn functionProfileComputationChunkNormalized(a: JsValue) -> Result<String, JsValue> {
    let a: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
        serde_wasm_bindgen::from_value(a)?;
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Converts an accumulated `AgentCompletionChunk` to an `AgentCompletion` (unary).
#[wasm_bindgen]
pub fn agentCompletionChunkToUnary(a: JsValue) -> Result<String, JsValue> {
    let a: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
        serde_wasm_bindgen::from_value(a)?;
    let unary: objectiveai_sdk::agent::completions::response::unary::AgentCompletion = a.into();
    serde_json::to_string(&unary).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Converts an accumulated `VectorCompletionChunk` to a `VectorCompletion` (unary).
#[wasm_bindgen]
pub fn vectorCompletionChunkToUnary(a: JsValue) -> Result<String, JsValue> {
    let a: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
        serde_wasm_bindgen::from_value(a)?;
    let unary: objectiveai_sdk::vector::completions::response::unary::VectorCompletion = a.into();
    serde_json::to_string(&unary).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Converts an accumulated `FunctionExecutionChunk` to a `FunctionExecution` (unary).
#[wasm_bindgen]
pub fn functionExecutionChunkToUnary(a: JsValue) -> Result<String, JsValue> {
    let a: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
        serde_wasm_bindgen::from_value(a)?;
    let unary: objectiveai_sdk::functions::executions::response::unary::FunctionExecution = a.into();
    serde_json::to_string(&unary).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Converts an accumulated `FunctionProfileComputationChunk` to a `FunctionProfileComputation` (unary).
#[wasm_bindgen]
pub fn functionProfileComputationChunkToUnary(a: JsValue) -> Result<String, JsValue> {
    let a: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
        serde_wasm_bindgen::from_value(a)?;
    let unary: objectiveai_sdk::functions::profiles::computations::response::unary::FunctionProfileComputation = a.into();
    serde_json::to_string(&unary).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Normalizes an `AgentCompletion` for test snapshot stability.
#[wasm_bindgen]
pub fn normalizeAgentCompletionForTests(a: JsValue) -> Result<String, JsValue> {
    let mut a: objectiveai_sdk::agent::completions::response::unary::AgentCompletion =
        serde_wasm_bindgen::from_value(a)?;
    a.normalize_for_tests();
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Normalizes a `VectorCompletion` for test snapshot stability.
#[wasm_bindgen]
pub fn normalizeVectorCompletionForTests(a: JsValue) -> Result<String, JsValue> {
    let mut a: objectiveai_sdk::vector::completions::response::unary::VectorCompletion =
        serde_wasm_bindgen::from_value(a)?;
    a.normalize_for_tests();
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Normalizes a `FunctionExecution` for test snapshot stability.
#[wasm_bindgen]
pub fn normalizeFunctionExecutionForTests(a: JsValue) -> Result<String, JsValue> {
    let mut a: objectiveai_sdk::functions::executions::response::unary::FunctionExecution =
        serde_wasm_bindgen::from_value(a)?;
    a.normalize_for_tests();
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Normalizes a `FunctionProfileComputation` for test snapshot stability.
#[wasm_bindgen]
pub fn normalizeFunctionProfileComputationForTests(a: JsValue) -> Result<String, JsValue> {
    let mut a: objectiveai_sdk::functions::profiles::computations::response::unary::FunctionProfileComputation =
        serde_wasm_bindgen::from_value(a)?;
    a.normalize_for_tests();
    serde_json::to_string(&a).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Generates a random `AgentCompletionChunk`. Optional seed for reproducibility.
#[wasm_bindgen]
pub fn generateAgentCompletionChunk(seed: JsValue) -> Result<String, JsValue> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk::arbitrary(&mut u)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&chunk).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Generates a random `VectorCompletionChunk`. Optional seed for reproducibility.
#[wasm_bindgen]
pub fn generateVectorCompletionChunk(seed: JsValue) -> Result<String, JsValue> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk::arbitrary(&mut u)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&chunk).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Generates a random `FunctionExecutionChunk`. Optional seed for reproducibility.
#[wasm_bindgen]
pub fn generateFunctionExecutionChunk(seed: JsValue) -> Result<String, JsValue> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk::arbitrary(&mut u)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&chunk).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Generates a random `FunctionProfileComputationChunk`. Optional seed for reproducibility.
#[wasm_bindgen]
pub fn generateFunctionProfileComputationChunk(seed: JsValue) -> Result<String, JsValue> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk::arbitrary(&mut u)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&chunk).map_err(|e| JsValue::from_str(&e.to_string()))
}
