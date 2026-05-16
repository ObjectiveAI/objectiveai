//! Python bindings for ObjectiveAI via PyO3.
//!
//! This crate provides the same functions as `objectiveai-sdk-rs-wasm-js` but for
//! Python instead of JavaScript. It uses `pythonize` for zero-copy conversion
//! between Python dicts and Rust serde types.

use arbitrary::Arbitrary;
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pythonize::{depythonize, pythonize};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deserialize a Python object into a Rust type via pythonize.
fn from_py<'py, T: serde::de::DeserializeOwned>(obj: &Bound<'py, PyAny>) -> PyResult<T> {
    depythonize(obj).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Serialize a Rust type into a Python object via pythonize.
fn to_py<T: serde::Serialize>(py: Python<'_>, val: &T) -> PyResult<Py<PyAny>> {
    pythonize(py, val)
        .map(|obj| obj.unbind())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

// ---------------------------------------------------------------------------
// Validation & ID Computation
// ---------------------------------------------------------------------------

/// Validates an Agent configuration and computes its content-addressed ID.
///
/// Returns the validated Agent as a Python dict with its computed `id` field.
#[pyfunction]
fn validate_agent(py: Python<'_>, agent: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let agent_base: objectiveai_sdk::agent::AgentBase = from_py(agent)?;
    let agent: objectiveai_sdk::agent::Agent = agent_base
        .convert()
        .map_err(|e| PyValueError::new_err(e))?;
    to_py(py, &agent)
}

/// Validates an Swarm configuration and computes its content-addressed ID.
///
/// Returns the validated Swarm as a Python dict with its computed `id` field.
#[pyfunction]
#[pyo3(signature = (swarm, remote_agents=None))]
fn validate_swarm(py: Python<'_>, swarm: &Bound<'_, PyAny>, remote_agents: Option<&Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let swarm_base: objectiveai_sdk::swarm::SwarmBase = from_py(swarm)?;
    let remote_agents: Option<std::collections::HashMap<String, objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>> =
        match remote_agents {
            Some(ra) => Some(from_py(ra)?),
            None => None,
        };
    let swarm: objectiveai_sdk::swarm::Swarm = swarm_base
        .convert(remote_agents.as_ref())
        .map_err(|e| PyValueError::new_err(e))?;
    to_py(py, &swarm)
}

/// Computes a content-addressed ID for chat messages.
///
/// Returns a base62-encoded hash string uniquely identifying the prompt content.
#[pyfunction]
fn prompt_id(prompt: &Bound<'_, PyAny>) -> PyResult<String> {
    let mut prompt: Vec<objectiveai_sdk::agent::completions::message::Message> = from_py(prompt)?;
    objectiveai_sdk::agent::completions::message::prompt::prepare(&mut prompt);
    Ok(objectiveai_sdk::agent::completions::message::prompt::id(&prompt))
}

/// Computes a content-addressed ID for a vector completion response option.
///
/// Returns a base62-encoded hash string uniquely identifying the response content.
#[pyfunction]
fn vector_response_id(response: &Bound<'_, PyAny>) -> PyResult<String> {
    let mut response: objectiveai_sdk::agent::completions::message::RichContent = from_py(response)?;
    response.prepare();
    Ok(response.id())
}

// ---------------------------------------------------------------------------
// Function Input Validation
// ---------------------------------------------------------------------------

/// Validates function input against its schema.
///
/// Returns True if valid, False if invalid, None for inline functions.
#[pyfunction]
fn validate_function_input(
    function: &Bound<'_, PyAny>,
    input: &Bound<'_, PyAny>,
) -> PyResult<Option<bool>> {
    let function: objectiveai_sdk::functions::Function = from_py(function)?;
    let input: objectiveai_sdk::functions::expression::InputValue = from_py(input)?;
    Ok(function.validate_input(&input))
}

// ---------------------------------------------------------------------------
// Function Task Compilation
// ---------------------------------------------------------------------------

/// Compiles a Function's task expressions for a given input.
///
/// Returns a list where each element is None (skipped), {"One": task}, or {"Many": [tasks]}.
#[pyfunction]
fn compile_function_tasks(
    py: Python<'_>,
    function: &Bound<'_, PyAny>,
    input: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let function: objectiveai_sdk::functions::Function = from_py(function)?;
    let input: objectiveai_sdk::functions::expression::InputValue = from_py(input)?;
    let tasks = function
        .compile_tasks(&input)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    to_py(py, &tasks)
}

/// Computes the expected output length for a vector Function.
///
/// Returns the expected output length, or None for scalar/inline functions.
#[pyfunction]
fn compile_function_output_length(
    function: &Bound<'_, PyAny>,
    input: &Bound<'_, PyAny>,
) -> PyResult<Option<u32>> {
    let function: objectiveai_sdk::functions::Function = from_py(function)?;
    let input: objectiveai_sdk::functions::expression::InputValue = from_py(input)?;
    Ok(function
        .compile_output_length(&input)
        .map_err(|e| PyValueError::new_err(e.to_string()))?
        .map(|u| u as u32))
}

/// Compiles the `input_split` expression to split input into sub-inputs.
///
/// Returns a list of split inputs, or None for scalar functions.
#[pyfunction]
fn compile_function_input_split(
    py: Python<'_>,
    function: &Bound<'_, PyAny>,
    input: &Bound<'_, PyAny>,
) -> PyResult<Option<Py<PyAny>>> {
    let function: objectiveai_sdk::functions::Function = from_py(function)?;
    let input: objectiveai_sdk::functions::expression::InputValue = from_py(input)?;
    let input_split = function
        .compile_input_split(&input)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    match input_split {
        Some(split) => Ok(Some(to_py(py, &split)?)),
        None => Ok(None),
    }
}

/// Compiles the `input_merge` expression to merge sub-inputs back into one.
///
/// Returns the merged input, or None for scalar functions.
#[pyfunction]
fn compile_function_input_merge(
    py: Python<'_>,
    function: &Bound<'_, PyAny>,
    input: &Bound<'_, PyAny>,
) -> PyResult<Option<Py<PyAny>>> {
    let function: objectiveai_sdk::functions::Function = from_py(function)?;
    let input: Vec<objectiveai_sdk::functions::expression::InputValue> = from_py(input)?;
    let input_merge = function
        .compile_input_merge(&objectiveai_sdk::functions::expression::InputValue::Array(input))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    match input_merge {
        Some(merge) => Ok(Some(to_py(py, &merge)?)),
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Vector/Scalar Field Validation
// ---------------------------------------------------------------------------

/// Validates vector function fields (output_length, input_split, input_merge).
#[pyfunction]
fn check_vector_fields(fields: &Bound<'_, PyAny>) -> PyResult<()> {
    let fields: objectiveai_sdk::functions::check::VectorFieldsValidation = from_py(fields)?;
    objectiveai_sdk::functions::check::check_vector_fields(fields, None)
        .map_err(|e| PyValueError::new_err(e))
}

/// Validates scalar function fields (input_schema only).
#[pyfunction]
fn check_scalar_fields(fields: &Bound<'_, PyAny>) -> PyResult<()> {
    let fields: objectiveai_sdk::functions::check::ScalarFieldsValidation = from_py(fields)?;
    objectiveai_sdk::functions::check::check_scalar_fields(fields, None)
        .map_err(|e| PyValueError::new_err(e))
}

// ---------------------------------------------------------------------------
// Alpha Function Validation
// ---------------------------------------------------------------------------

/// Alpha check for a leaf scalar function (depth 0, scalar output).
#[pyfunction]
fn alpha_check_leaf_scalar_function(function: &Bound<'_, PyAny>) -> PyResult<()> {
    let function: objectiveai_sdk::functions::alpha_scalar::RemoteFunction = from_py(function)?;
    objectiveai_sdk::functions::alpha_scalar::check::check_alpha_leaf_scalar_function(&function, None)
        .map_err(|e| PyValueError::new_err(e))
}

/// Alpha check for a branch scalar function (depth > 0, scalar output).
#[pyfunction]
fn alpha_check_branch_scalar_function(
    function: &Bound<'_, PyAny>,
    children: Option<&Bound<'_, PyAny>>,
) -> PyResult<()> {
    let function: objectiveai_sdk::functions::alpha_scalar::RemoteFunction = from_py(function)?;
    let children: Option<std::collections::HashMap<String, objectiveai_sdk::functions::FullRemoteFunction>> =
        match children {
            Some(c) => Some(from_py(c)?),
            None => None,
        };
    objectiveai_sdk::functions::alpha_scalar::check::check_alpha_branch_scalar_function(
        &function,
        children.as_ref(),
        None,
    )
    .map_err(|e| PyValueError::new_err(e))
}

/// Alpha check for a leaf vector function (depth 0, vector output).
#[pyfunction]
fn alpha_check_leaf_vector_function(function: &Bound<'_, PyAny>) -> PyResult<()> {
    let function: objectiveai_sdk::functions::alpha_vector::RemoteFunction = from_py(function)?;
    objectiveai_sdk::functions::alpha_vector::check::check_alpha_leaf_vector_function(&function, None)
        .map_err(|e| PyValueError::new_err(e))
}

/// Alpha check for a branch vector function (depth > 0, vector output).
#[pyfunction]
fn alpha_check_branch_vector_function(
    function: &Bound<'_, PyAny>,
    children: Option<&Bound<'_, PyAny>>,
) -> PyResult<()> {
    let function: objectiveai_sdk::functions::alpha_vector::RemoteFunction = from_py(function)?;
    let children: Option<std::collections::HashMap<String, objectiveai_sdk::functions::FullRemoteFunction>> =
        match children {
            Some(c) => Some(from_py(c)?),
            None => None,
        };
    objectiveai_sdk::functions::alpha_vector::check::check_alpha_branch_vector_function(
        &function,
        children.as_ref(),
        None,
    )
    .map_err(|e| PyValueError::new_err(e))
}

// ---------------------------------------------------------------------------
// Streaming Chunk Merging
// ---------------------------------------------------------------------------

/// Merges two AgentCompletionChunks via push and returns the merged result.
#[pyfunction]
fn agent_completion_chunk_merged(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
        from_py(a)?;
    let b: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
        from_py(b)?;
    a.push(&b);
    to_py(py, &a)
}

/// Merges two VectorCompletionChunks via push and returns the merged result.
#[pyfunction]
fn vector_completion_chunk_merged(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
        from_py(a)?;
    let b: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
        from_py(b)?;
    a.push(&b);
    to_py(py, &a)
}

/// Merges two FunctionExecutionChunks via push and returns the merged result.
#[pyfunction]
fn function_execution_chunk_merged(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
        from_py(a)?;
    let b: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
        from_py(b)?;
    a.push(&b);
    to_py(py, &a)
}

/// Merges two FunctionInventionChunks via push and returns the merged result.
#[pyfunction]
fn function_invention_chunk_merged(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk =
        from_py(a)?;
    let b: objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk =
        from_py(b)?;
    a.push(&b);
    to_py(py, &a)
}

/// Merges two FunctionInventionRecursiveChunks via push and returns the merged result.
#[pyfunction]
fn function_invention_recursive_chunk_merged(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk =
        from_py(a)?;
    let b: objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk =
        from_py(b)?;
    a.push(&b);
    to_py(py, &a)
}

/// Merges two FunctionProfileComputationChunks via push and returns the merged result.
#[pyfunction]
fn function_profile_computation_chunk_merged(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
        from_py(a)?;
    let b: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
        from_py(b)?;
    a.push(&b);
    to_py(py, &a)
}

/// Merges two LaboratoryExecutionChunks via push and returns the merged result.
#[pyfunction]
fn laboratory_execution_chunk_merged(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk =
        from_py(a)?;
    let b: objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk =
        from_py(b)?;
    a.push(&b);
    to_py(py, &a)
}

// ---------------------------------------------------------------------------
// Streaming Chunk Normalization
// ---------------------------------------------------------------------------

/// Normalizes an AgentCompletionChunk by round-tripping through serde.
#[pyfunction]
fn agent_completion_chunk_normalized(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
        from_py(a)?;
    to_py(py, &a)
}

/// Normalizes a VectorCompletionChunk by round-tripping through serde.
#[pyfunction]
fn vector_completion_chunk_normalized(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
        from_py(a)?;
    to_py(py, &a)
}

/// Normalizes a FunctionExecutionChunk by round-tripping through serde.
#[pyfunction]
fn function_execution_chunk_normalized(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
        from_py(a)?;
    to_py(py, &a)
}

/// Normalizes a FunctionInventionChunk by round-tripping through serde.
#[pyfunction]
fn function_invention_chunk_normalized(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk =
        from_py(a)?;
    to_py(py, &a)
}

/// Normalizes a FunctionInventionRecursiveChunk by round-tripping through serde.
#[pyfunction]
fn function_invention_recursive_chunk_normalized(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk =
        from_py(a)?;
    to_py(py, &a)
}

/// Normalizes a FunctionProfileComputationChunk by round-tripping through serde.
#[pyfunction]
fn function_profile_computation_chunk_normalized(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
        from_py(a)?;
    to_py(py, &a)
}

/// Normalizes a LaboratoryExecutionChunk by round-tripping through serde.
#[pyfunction]
fn laboratory_execution_chunk_normalized(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk =
        from_py(a)?;
    to_py(py, &a)
}

// ---------------------------------------------------------------------------
// Streaming Chunk to Unary Conversion
// ---------------------------------------------------------------------------

/// Converts an accumulated AgentCompletionChunk to an AgentCompletion (unary).
#[pyfunction]
fn agent_completion_chunk_to_unary(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
        from_py(a)?;
    let unary: objectiveai_sdk::agent::completions::response::unary::AgentCompletion = a.into();
    to_py(py, &unary)
}

/// Converts an accumulated VectorCompletionChunk to a VectorCompletion (unary).
#[pyfunction]
fn vector_completion_chunk_to_unary(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
        from_py(a)?;
    let unary: objectiveai_sdk::vector::completions::response::unary::VectorCompletion = a.into();
    to_py(py, &unary)
}

/// Converts an accumulated FunctionExecutionChunk to a FunctionExecution (unary).
#[pyfunction]
fn function_execution_chunk_to_unary(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
        from_py(a)?;
    let unary: objectiveai_sdk::functions::executions::response::unary::FunctionExecution = a.into();
    to_py(py, &unary)
}

/// Converts an accumulated FunctionInventionChunk to a FunctionInvention (unary).
#[pyfunction]
fn function_invention_chunk_to_unary(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk =
        from_py(a)?;
    let unary: objectiveai_sdk::functions::inventions::response::unary::FunctionInvention = a.into();
    to_py(py, &unary)
}

/// Converts an accumulated FunctionInventionRecursiveChunk to a FunctionInventionRecursive (unary).
#[pyfunction]
fn function_invention_recursive_chunk_to_unary(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk =
        from_py(a)?;
    let unary: objectiveai_sdk::functions::inventions::recursive::response::unary::FunctionInventionRecursive = a.into();
    to_py(py, &unary)
}

/// Converts an accumulated FunctionProfileComputationChunk to a FunctionProfileComputation (unary).
#[pyfunction]
fn function_profile_computation_chunk_to_unary(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
        from_py(a)?;
    let unary: objectiveai_sdk::functions::profiles::computations::response::unary::FunctionProfileComputation = a.into();
    to_py(py, &unary)
}

/// Converts an accumulated LaboratoryExecutionChunk to a LaboratoryExecution (unary).
#[pyfunction]
fn laboratory_execution_chunk_to_unary(
    py: Python<'_>,
    a: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let a: objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk =
        from_py(a)?;
    let unary: objectiveai_sdk::laboratories::executions::response::unary::LaboratoryExecution = a.into();
    to_py(py, &unary)
}

// ---------------------------------------------------------------------------
// Normalize for tests
// ---------------------------------------------------------------------------

/// Normalizes an AgentCompletion for test snapshot stability.
#[pyfunction]
fn normalize_agent_completion_for_tests(py: Python<'_>, a: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::agent::completions::response::unary::AgentCompletion = from_py(a)?;
    a.normalize_for_tests();
    to_py(py, &a)
}

/// Normalizes a VectorCompletion for test snapshot stability.
#[pyfunction]
fn normalize_vector_completion_for_tests(py: Python<'_>, a: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::vector::completions::response::unary::VectorCompletion = from_py(a)?;
    a.normalize_for_tests();
    to_py(py, &a)
}

/// Normalizes a FunctionExecution for test snapshot stability.
#[pyfunction]
fn normalize_function_execution_for_tests(py: Python<'_>, a: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::functions::executions::response::unary::FunctionExecution = from_py(a)?;
    a.normalize_for_tests();
    to_py(py, &a)
}

/// Normalizes a FunctionInvention for test snapshot stability.
#[pyfunction]
fn normalize_function_invention_for_tests(py: Python<'_>, a: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::functions::inventions::response::unary::FunctionInvention = from_py(a)?;
    a.normalize_for_tests();
    to_py(py, &a)
}

/// Normalizes a FunctionInventionRecursive for test snapshot stability.
#[pyfunction]
fn normalize_function_invention_recursive_for_tests(py: Python<'_>, a: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::functions::inventions::recursive::response::unary::FunctionInventionRecursive = from_py(a)?;
    a.normalize_for_tests();
    to_py(py, &a)
}

/// Normalizes a FunctionProfileComputation for test snapshot stability.
#[pyfunction]
fn normalize_function_profile_computation_for_tests(py: Python<'_>, a: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::functions::profiles::computations::response::unary::FunctionProfileComputation = from_py(a)?;
    a.normalize_for_tests();
    to_py(py, &a)
}

/// Normalizes a LaboratoryExecution for test snapshot stability.
#[pyfunction]
fn normalize_laboratory_execution_for_tests(py: Python<'_>, a: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let mut a: objectiveai_sdk::laboratories::executions::response::unary::LaboratoryExecution = from_py(a)?;
    a.normalize_for_tests();
    to_py(py, &a)
}

// ---------------------------------------------------------------------------
// Seed → bytes helper
// ---------------------------------------------------------------------------

fn seed_to_bytes(seed: Option<i64>) -> Vec<u8> {
    use rand::prelude::*;

    let seed_val: u64 = match seed {
        Some(s) => s as u64,
        None => rand::random(),
    };

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed_val);
    let mut bytes = vec![0u8; 4096];
    rng.fill_bytes(&mut bytes);
    bytes
}

// ---------------------------------------------------------------------------
// Generate arbitrary chunks
// ---------------------------------------------------------------------------

/// Generates a random AgentCompletionChunk. Optional seed for reproducibility.
#[pyfunction]
#[pyo3(signature = (seed=None))]
fn generate_agent_completion_chunk(py: Python<'_>, seed: Option<i64>) -> PyResult<Py<PyAny>> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk::arbitrary(&mut u)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    to_py(py, &chunk)
}

/// Generates a random VectorCompletionChunk. Optional seed for reproducibility.
#[pyfunction]
#[pyo3(signature = (seed=None))]
fn generate_vector_completion_chunk(py: Python<'_>, seed: Option<i64>) -> PyResult<Py<PyAny>> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk::arbitrary(&mut u)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    to_py(py, &chunk)
}

/// Generates a random FunctionExecutionChunk. Optional seed for reproducibility.
#[pyfunction]
#[pyo3(signature = (seed=None))]
fn generate_function_execution_chunk(py: Python<'_>, seed: Option<i64>) -> PyResult<Py<PyAny>> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk::arbitrary(&mut u)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    to_py(py, &chunk)
}

/// Generates a random FunctionInventionChunk. Optional seed for reproducibility.
#[pyfunction]
#[pyo3(signature = (seed=None))]
fn generate_function_invention_chunk(py: Python<'_>, seed: Option<i64>) -> PyResult<Py<PyAny>> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk::arbitrary(&mut u)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    to_py(py, &chunk)
}

/// Generates a random FunctionInventionRecursiveChunk. Optional seed for reproducibility.
#[pyfunction]
#[pyo3(signature = (seed=None))]
fn generate_function_invention_recursive_chunk(py: Python<'_>, seed: Option<i64>) -> PyResult<Py<PyAny>> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk::arbitrary(&mut u)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    to_py(py, &chunk)
}

/// Generates a random FunctionProfileComputationChunk. Optional seed for reproducibility.
#[pyfunction]
#[pyo3(signature = (seed=None))]
fn generate_function_profile_computation_chunk(py: Python<'_>, seed: Option<i64>) -> PyResult<Py<PyAny>> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk::arbitrary(&mut u)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    to_py(py, &chunk)
}

/// Generates a random LaboratoryExecutionChunk. Optional seed for reproducibility.
#[pyfunction]
#[pyo3(signature = (seed=None))]
fn generate_laboratory_execution_chunk(py: Python<'_>, seed: Option<i64>) -> PyResult<Py<PyAny>> {
    let bytes = seed_to_bytes(seed);
    let mut u = arbitrary::Unstructured::new(&bytes);
    let chunk = objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk::arbitrary(&mut u)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    to_py(py, &chunk)
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

#[pymodule]
fn _pyo3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Validation & ID
    m.add_function(wrap_pyfunction!(validate_agent, m)?)?;
    m.add_function(wrap_pyfunction!(validate_swarm, m)?)?;
    m.add_function(wrap_pyfunction!(prompt_id, m)?)?;
    m.add_function(wrap_pyfunction!(vector_response_id, m)?)?;

    // Function input validation
    m.add_function(wrap_pyfunction!(validate_function_input, m)?)?;

    // Function compilation
    m.add_function(wrap_pyfunction!(compile_function_tasks, m)?)?;
    m.add_function(wrap_pyfunction!(compile_function_output_length, m)?)?;
    m.add_function(wrap_pyfunction!(compile_function_input_split, m)?)?;
    m.add_function(wrap_pyfunction!(compile_function_input_merge, m)?)?;

    // Field validation
    m.add_function(wrap_pyfunction!(check_vector_fields, m)?)?;
    m.add_function(wrap_pyfunction!(check_scalar_fields, m)?)?;

    // Alpha function validation
    m.add_function(wrap_pyfunction!(alpha_check_leaf_scalar_function, m)?)?;
    m.add_function(wrap_pyfunction!(alpha_check_branch_scalar_function, m)?)?;
    m.add_function(wrap_pyfunction!(alpha_check_leaf_vector_function, m)?)?;
    m.add_function(wrap_pyfunction!(alpha_check_branch_vector_function, m)?)?;

    // Streaming chunk merging
    m.add_function(wrap_pyfunction!(agent_completion_chunk_merged, m)?)?;
    m.add_function(wrap_pyfunction!(vector_completion_chunk_merged, m)?)?;
    m.add_function(wrap_pyfunction!(function_execution_chunk_merged, m)?)?;
    m.add_function(wrap_pyfunction!(function_invention_chunk_merged, m)?)?;
    m.add_function(wrap_pyfunction!(function_invention_recursive_chunk_merged, m)?)?;
    m.add_function(wrap_pyfunction!(function_profile_computation_chunk_merged, m)?)?;
    m.add_function(wrap_pyfunction!(laboratory_execution_chunk_merged, m)?)?;

    // Streaming chunk normalization
    m.add_function(wrap_pyfunction!(agent_completion_chunk_normalized, m)?)?;
    m.add_function(wrap_pyfunction!(vector_completion_chunk_normalized, m)?)?;
    m.add_function(wrap_pyfunction!(function_execution_chunk_normalized, m)?)?;
    m.add_function(wrap_pyfunction!(function_invention_chunk_normalized, m)?)?;
    m.add_function(wrap_pyfunction!(function_invention_recursive_chunk_normalized, m)?)?;
    m.add_function(wrap_pyfunction!(function_profile_computation_chunk_normalized, m)?)?;
    m.add_function(wrap_pyfunction!(laboratory_execution_chunk_normalized, m)?)?;

    // Streaming chunk to unary conversion
    m.add_function(wrap_pyfunction!(agent_completion_chunk_to_unary, m)?)?;
    m.add_function(wrap_pyfunction!(vector_completion_chunk_to_unary, m)?)?;
    m.add_function(wrap_pyfunction!(function_execution_chunk_to_unary, m)?)?;
    m.add_function(wrap_pyfunction!(function_invention_chunk_to_unary, m)?)?;
    m.add_function(wrap_pyfunction!(function_invention_recursive_chunk_to_unary, m)?)?;
    m.add_function(wrap_pyfunction!(function_profile_computation_chunk_to_unary, m)?)?;
    m.add_function(wrap_pyfunction!(laboratory_execution_chunk_to_unary, m)?)?;

    // Normalize for tests
    m.add_function(wrap_pyfunction!(normalize_agent_completion_for_tests, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_vector_completion_for_tests, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_function_execution_for_tests, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_function_invention_for_tests, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_function_invention_recursive_for_tests, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_function_profile_computation_for_tests, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_laboratory_execution_for_tests, m)?)?;

    // Generate arbitrary chunks
    m.add_function(wrap_pyfunction!(generate_agent_completion_chunk, m)?)?;
    m.add_function(wrap_pyfunction!(generate_vector_completion_chunk, m)?)?;
    m.add_function(wrap_pyfunction!(generate_function_execution_chunk, m)?)?;
    m.add_function(wrap_pyfunction!(generate_function_invention_chunk, m)?)?;
    m.add_function(wrap_pyfunction!(generate_function_invention_recursive_chunk, m)?)?;
    m.add_function(wrap_pyfunction!(generate_function_profile_computation_chunk, m)?)?;
    m.add_function(wrap_pyfunction!(generate_laboratory_execution_chunk, m)?)?;

    Ok(())
}
