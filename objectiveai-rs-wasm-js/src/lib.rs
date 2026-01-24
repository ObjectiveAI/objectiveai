//! WebAssembly bindings for ObjectiveAI.
//!
//! This crate provides JavaScript/TypeScript bindings for client-side validation
//! and compilation of ObjectiveAI types. It enables browser-based applications to:
//!
//! - Validate Ensemble LLM and Ensemble configurations
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
//! - [`validateEnsembleLlm`] - Validate and compute ID for an Ensemble LLM
//! - [`validateEnsemble`] - Validate and compute ID for an Ensemble
//! - [`compileFunctionTasks`] - Compile function tasks for a given input
//! - [`compileFunctionOutput`] - Compile function output from task results
//! - [`promptId`] - Compute content-addressed ID for chat messages
//! - [`toolsId`] - Compute content-addressed ID for tools
//! - [`vectorResponseId`] - Compute content-addressed ID for a response option

#![allow(non_snake_case)]
use wasm_bindgen::prelude::*;

/// Validates an Ensemble LLM configuration and computes its content-addressed ID.
///
/// Takes an Ensemble LLM definition, normalizes it (removes defaults, deduplicates),
/// validates all fields, and computes a deterministic ID using XXHash3-128.
///
/// # Arguments
///
/// * `llm` - JavaScript object representing an Ensemble LLM configuration
///
/// # Returns
///
/// The validated Ensemble LLM with its computed `id` field populated.
///
/// # Errors
///
/// Returns an error string if validation fails (e.g., invalid model name,
/// out-of-range parameters, conflicting settings).
#[wasm_bindgen]
pub fn validateEnsembleLlm(llm: JsValue) -> Result<JsValue, JsValue> {
    // deserialize
    let llm_base: objectiveai::ensemble_llm::EnsembleLlmBase =
        serde_wasm_bindgen::from_value(llm)?;
    // prepare, validate, and compute ID
    let llm: objectiveai::ensemble_llm::EnsembleLlm = llm_base
        .try_into()
        .map_err(|e: String| JsValue::from_str(&e))?;
    // serialize
    let llm: JsValue = serde_wasm_bindgen::to_value(&llm)?;
    Ok(llm)
}

/// Validates an Ensemble configuration and computes its content-addressed ID.
///
/// Takes an Ensemble definition (a collection of Ensemble LLMs), validates each
/// LLM, and computes a deterministic ID for the ensemble as a whole.
///
/// # Arguments
///
/// * `ensemble` - JavaScript object representing an Ensemble configuration
///
/// # Returns
///
/// The validated Ensemble with its computed `id` field populated and all
/// member LLMs validated with their IDs.
///
/// # Errors
///
/// Returns an error string if any LLM validation fails or the ensemble
/// structure is invalid.
#[wasm_bindgen]
pub fn validateEnsemble(ensemble: JsValue) -> Result<JsValue, JsValue> {
    // deserialize
    let ensemble_base: objectiveai::ensemble::EnsembleBase =
        serde_wasm_bindgen::from_value(ensemble)?;
    // prepare, validate, and compute ID
    let ensemble: objectiveai::ensemble::Ensemble = ensemble_base
        .try_into()
        .map_err(|e: String| JsValue::from_str(&e))?;
    // serialize
    let ensemble: JsValue = serde_wasm_bindgen::to_value(&ensemble)?;
    Ok(ensemble)
}

/// Compiles a Function's task expressions for a given input.
///
/// Evaluates all JMESPath expressions in the function's tasks using the
/// provided input data. This is used for previewing how tasks will be
/// executed during Function authoring.
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
/// - `{ Many: [task, ...] }` for mapped tasks (expanded from input_maps)
///
/// # Errors
///
/// Returns an error string if expression evaluation fails or types don't match.
#[wasm_bindgen]
pub fn compileFunctionTasks(
    function: JsValue,
    input: JsValue,
) -> Result<JsValue, JsValue> {
    // deserialize
    let function: objectiveai::functions::Function =
        serde_wasm_bindgen::from_value(function)?;
    let input: objectiveai::functions::expression::Input =
        serde_wasm_bindgen::from_value(input)?;
    // compile tasks
    let tasks = function
        .compile_tasks(&input)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    // serialize
    let tasks: JsValue = serde_wasm_bindgen::to_value(&tasks)?;
    Ok(tasks)
}

/// Computes the final output of a Function given input and task results.
///
/// Evaluates the function's output expression using the provided input data
/// and task outputs. Also validates that the output meets constraints:
/// - Scalar functions: output must be in [0, 1]
/// - Vector functions: output must sum to approximately 1
///
/// # Arguments
///
/// * `function` - JavaScript object representing a Function definition
/// * `input` - JavaScript object representing the function input
/// * `task_outputs` - Array of task outputs (from actual execution or mocked)
///
/// # Returns
///
/// An object with:
/// - `output`: The computed scalar or vector output
/// - `valid`: Boolean indicating if the output meets constraints
///
/// # Errors
///
/// Returns an error string if expression evaluation fails.
#[wasm_bindgen]
pub fn compileFunctionOutput(
    function: JsValue,
    input: JsValue,
    task_outputs: JsValue,
) -> Result<JsValue, JsValue> {
    // deserialize
    let function: objectiveai::functions::Function =
        serde_wasm_bindgen::from_value(function)?;
    let input: objectiveai::functions::expression::Input =
        serde_wasm_bindgen::from_value(input)?;
    let task_outputs: Vec<
        Option<objectiveai::functions::expression::TaskOutput<'static>>,
    > = serde_wasm_bindgen::from_value(task_outputs)?;
    // compile output
    let output = function
        .compile_output(&input, &task_outputs)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    // serialize
    let output: JsValue = serde_wasm_bindgen::to_value(&output)?;
    Ok(output)
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
    let mut prompt: Vec<objectiveai::chat::completions::request::Message> =
        serde_wasm_bindgen::from_value(prompt)?;
    // prepare and compute ID
    objectiveai::chat::completions::request::prompt::prepare(&mut prompt);
    let id = objectiveai::chat::completions::request::prompt::id(&prompt);
    Ok(id)
}

/// Computes a content-addressed ID for a tools array.
///
/// Computes a deterministic hash for the tools configuration. This ID is
/// used for caching and deduplicating requests with identical tool sets.
///
/// # Arguments
///
/// * `tools` - Array of tool definitions
///
/// # Returns
///
/// A base62-encoded hash string uniquely identifying the tools.
///
/// # Errors
///
/// Returns an error if the tools cannot be deserialized.
#[wasm_bindgen]
pub fn toolsId(tools: JsValue) -> Result<String, JsValue> {
    // deserialize
    let tools: Vec<objectiveai::chat::completions::request::Tool> =
        serde_wasm_bindgen::from_value(tools)?;
    // compute ID
    let id = objectiveai::chat::completions::request::tools::id(&tools);
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
    let mut response: objectiveai::chat::completions::request::RichContent =
        serde_wasm_bindgen::from_value(response)?;
    // prepare and compute ID
    response.prepare();
    let id = response.id();
    Ok(id)
}
