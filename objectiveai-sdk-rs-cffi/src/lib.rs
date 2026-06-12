//! C FFI bindings for ObjectiveAI.
//!
//! This crate provides the same functions as `objectiveai-sdk-rs-wasm-js` and
//! `objectiveai-sdk-rs-pyo3` but via a C ABI, suitable for consumption by Go (CGo),
//! .NET (P/Invoke), and any other language with C FFI support.
//!
//! # ABI Convention
//!
//! All functions follow the same pattern:
//!
//! - **Input:** JSON bytes as `*const u8` + `usize` length
//! - **Output:** JSON bytes written to caller-provided `*mut *mut u8` + `*mut usize`
//! - **Return:** `0` on success, `-1` on error (error message written to output)
//! - **Memory:** Output is allocated by Rust. Caller must free it with [`objectiveai_free`].
//!
//! Functions that return `Option` (e.g., `validate_function_input`) use a separate
//! convention documented on each function.

use arbitrary::Arbitrary;
use std::slice;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deserialize JSON bytes into a Rust type.
fn from_json<T: serde::de::DeserializeOwned>(ptr: *const u8, len: usize) -> Result<T, String> {
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}

/// Serialize a Rust type to JSON bytes and write to output pointers.
///
/// Returns the JSON bytes as a Vec (caller writes to out pointers).
fn to_json<T: serde::Serialize>(val: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(val).map_err(|e| e.to_string())
}

/// Write a byte buffer to the output pointers. Caller must free with `objectiveai_free`.
unsafe fn write_output(out_ptr: *mut *mut u8, out_len: *mut usize, data: Vec<u8>) {
    let len = data.len();
    let boxed = data.into_boxed_slice();
    let ptr = Box::into_raw(boxed) as *mut u8;
    unsafe {
        *out_ptr = ptr;
        *out_len = len;
    }
}

/// Run a fallible operation that produces JSON output bytes.
/// On success, writes JSON to output and returns 0.
/// On error, writes error message to output and returns -1.
unsafe fn run(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    f: impl FnOnce() -> Result<Vec<u8>, String>,
) -> i32 {
    match f() {
        Ok(data) => {
            unsafe { write_output(out_ptr, out_len, data) };
            0
        }
        Err(e) => {
            unsafe { write_output(out_ptr, out_len, e.into_bytes()) };
            -1
        }
    }
}

// ---------------------------------------------------------------------------
// Memory Management
// ---------------------------------------------------------------------------

/// Allocates `len` bytes and returns a pointer to the allocation.
///
/// Used by WASM hosts to allocate memory in the WASM linear memory
/// for writing input data before calling FFI functions.
#[unsafe(no_mangle)]
pub extern "C" fn objectiveai_allocate(len: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Frees memory allocated by ObjectiveAI FFI functions.
///
/// Must be called on every non-null output pointer returned by any function
/// in this library, and on pointers returned by [`objectiveai_allocate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe {
            let _ = Box::from_raw(slice::from_raw_parts_mut(ptr, len));
        }
    }
}

// ---------------------------------------------------------------------------
// Validation & ID Computation
// ---------------------------------------------------------------------------

/// Validates an Agent configuration and computes its content-addressed ID.
///
/// Input: JSON bytes of an AgentBase.
/// Output: JSON bytes of the validated Agent with computed `id` field.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_validate_agent(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let base: objectiveai_sdk::agent::AgentBase = from_json(json_in, json_in_len)?;
            let agent: objectiveai_sdk::agent::Agent = base.convert()?;
            to_json(&agent)
        })
    }
}

/// Validates an Swarm configuration and computes its content-addressed ID.
///
/// Input: JSON bytes of an SwarmBase.
/// Optional: JSON bytes of a remote agents hashmap (pass null/0 if none).
/// Output: JSON bytes of the validated Swarm with computed `id` field.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_validate_swarm(
    json_in: *const u8,
    json_in_len: usize,
    remote_agents_in: *const u8,
    remote_agents_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let base: objectiveai_sdk::swarm::SwarmBase = from_json(json_in, json_in_len)?;
            // Values are `(base, path)` tuples to match
            // `SwarmBase::convert`'s signature; the RemotePath half is
            // ignored by the conversion (it resolves via `.0`).
            let remote_agents: Option<
                std::collections::HashMap<
                    String,
                    (objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks, objectiveai_sdk::RemotePath),
                >,
            > = if remote_agents_in.is_null() || remote_agents_in_len == 0 {
                None
            } else {
                Some(from_json(remote_agents_in, remote_agents_in_len)?)
            };
            let swarm: objectiveai_sdk::swarm::Swarm = base.convert(remote_agents.as_ref())?;
            to_json(&swarm)
        })
    }
}

/// Computes a content-addressed ID for chat messages.
///
/// Input: JSON bytes of an array of Messages.
/// Output: The base62-encoded hash string as UTF-8 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_prompt_id(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut prompt: Vec<objectiveai_sdk::agent::completions::message::Message> =
                from_json(json_in, json_in_len)?;
            objectiveai_sdk::agent::completions::message::prompt::prepare(&mut prompt);
            let id = objectiveai_sdk::agent::completions::message::prompt::id(&prompt);
            Ok(id.into_bytes())
        })
    }
}

/// Computes a content-addressed ID for a vector completion response option.
///
/// Input: JSON bytes of a RichContent.
/// Output: The base62-encoded hash string as UTF-8 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_vector_response_id(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut response: objectiveai_sdk::agent::completions::message::RichContent =
                from_json(json_in, json_in_len)?;
            response.prepare();
            let id = response.id();
            Ok(id.into_bytes())
        })
    }
}

// ---------------------------------------------------------------------------
// Function Input Validation
// ---------------------------------------------------------------------------

/// Validates function input against its schema.
///
/// Input: Two JSON buffers — function definition, then input value.
/// Returns: 1 if valid, 0 if invalid, 2 if not applicable (inline function), -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_validate_function_input(
    function_in: *const u8,
    function_in_len: usize,
    input_in: *const u8,
    input_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    let result = (|| -> Result<Option<bool>, String> {
        let function: objectiveai_sdk::functions::Function =
            from_json(function_in, function_in_len)?;
        let input: objectiveai_sdk::functions::expression::InputValue =
            from_json(input_in, input_in_len)?;
        Ok(function.validate_input(&input))
    })();

    unsafe {
        match result {
            Ok(Some(true)) => {
                write_output(json_out, json_out_len, Vec::new());
                1
            }
            Ok(Some(false)) => {
                write_output(json_out, json_out_len, Vec::new());
                0
            }
            Ok(None) => {
                write_output(json_out, json_out_len, Vec::new());
                2
            }
            Err(e) => {
                write_output(json_out, json_out_len, e.into_bytes());
                -1
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Function Task Compilation
// ---------------------------------------------------------------------------

/// Compiles a Function's task expressions for a given input.
///
/// Input: Two JSON buffers — function definition, then input value.
/// Output: JSON array of compiled tasks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_compile_function_tasks(
    function_in: *const u8,
    function_in_len: usize,
    input_in: *const u8,
    input_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let function: objectiveai_sdk::functions::Function =
                from_json(function_in, function_in_len)?;
            let input: objectiveai_sdk::functions::expression::InputValue =
                from_json(input_in, input_in_len)?;
            let tasks = function.compile_tasks(&input).map_err(|e| e.to_string())?;
            to_json(&tasks)
        })
    }
}

/// Computes the expected output length for a vector Function.
///
/// Input: Two JSON buffers — function definition, then input value.
/// Output: JSON number (u32) or "null".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_compile_function_output_length(
    function_in: *const u8,
    function_in_len: usize,
    input_in: *const u8,
    input_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let function: objectiveai_sdk::functions::Function =
                from_json(function_in, function_in_len)?;
            let input: objectiveai_sdk::functions::expression::InputValue =
                from_json(input_in, input_in_len)?;
            let len = function
                .compile_output_length(&input)
                .map_err(|e| e.to_string())?
                .map(|u| u as u32);
            to_json(&len)
        })
    }
}

/// Compiles the `input_split` expression.
///
/// Input: Two JSON buffers — function definition, then input value.
/// Output: JSON array of split inputs, or "null".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_compile_function_input_split(
    function_in: *const u8,
    function_in_len: usize,
    input_in: *const u8,
    input_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let function: objectiveai_sdk::functions::Function =
                from_json(function_in, function_in_len)?;
            let input: objectiveai_sdk::functions::expression::InputValue =
                from_json(input_in, input_in_len)?;
            let split = function
                .compile_input_split(&input)
                .map_err(|e| e.to_string())?;
            to_json(&split)
        })
    }
}

/// Compiles the `input_merge` expression.
///
/// Input: Two JSON buffers — function definition, then array of input values.
/// Output: JSON merged input, or "null".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_compile_function_input_merge(
    function_in: *const u8,
    function_in_len: usize,
    input_in: *const u8,
    input_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let function: objectiveai_sdk::functions::Function =
                from_json(function_in, function_in_len)?;
            let input: Vec<objectiveai_sdk::functions::expression::InputValue> =
                from_json(input_in, input_in_len)?;
            let merge = function
                .compile_input_merge(&objectiveai_sdk::functions::expression::InputValue::Array(input))
                .map_err(|e| e.to_string())?;
            to_json(&merge)
        })
    }
}

// ---------------------------------------------------------------------------
// Vector/Scalar Field Validation
// ---------------------------------------------------------------------------

/// Validates vector function fields (output_length, input_split, input_merge).
///
/// Input: JSON bytes of a VectorFieldsValidation.
/// Output: Empty on success. Error message on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_check_vector_fields(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let fields: objectiveai_sdk::functions::check::VectorFieldsValidation =
                from_json(json_in, json_in_len)?;
            objectiveai_sdk::functions::check::check_vector_fields(fields, None).map_err(|e| e)?;
            Ok(Vec::new())
        })
    }
}

/// Validates scalar function fields (input_schema only).
///
/// Input: JSON bytes of a ScalarFieldsValidation.
/// Output: Empty on success. Error message on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_check_scalar_fields(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let fields: objectiveai_sdk::functions::check::ScalarFieldsValidation =
                from_json(json_in, json_in_len)?;
            objectiveai_sdk::functions::check::check_scalar_fields(fields, None).map_err(|e| e)?;
            Ok(Vec::new())
        })
    }
}

// ---------------------------------------------------------------------------
// Alpha Function Validation
// ---------------------------------------------------------------------------

/// Alpha check for a leaf scalar function (depth 0, scalar output).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_alpha_check_leaf_scalar_function(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let function: objectiveai_sdk::functions::alpha_scalar::RemoteFunction =
                from_json(json_in, json_in_len)?;
            objectiveai_sdk::functions::alpha_scalar::check::check_alpha_leaf_scalar_function(
                &function, None,
            )
            .map_err(|e| e)?;
            Ok(Vec::new())
        })
    }
}

/// Alpha check for a branch scalar function (depth > 0, scalar output).
///
/// Input: Two JSON buffers — function definition, then optional children map.
/// Pass null/0 for children_in/children_in_len if no children.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_alpha_check_branch_scalar_function(
    function_in: *const u8,
    function_in_len: usize,
    children_in: *const u8,
    children_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let function: objectiveai_sdk::functions::alpha_scalar::RemoteFunction =
                from_json(function_in, function_in_len)?;
            let children: Option<
                std::collections::HashMap<String, objectiveai_sdk::functions::FullRemoteFunction>,
            > = if children_in.is_null() || children_in_len == 0 {
                None
            } else {
                Some(from_json(children_in, children_in_len)?)
            };
            objectiveai_sdk::functions::alpha_scalar::check::check_alpha_branch_scalar_function(
                &function,
                children.as_ref(),
                None,
            )
            .map_err(|e| e)?;
            Ok(Vec::new())
        })
    }
}

/// Alpha check for a leaf vector function (depth 0, vector output).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_alpha_check_leaf_vector_function(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let function: objectiveai_sdk::functions::alpha_vector::RemoteFunction =
                from_json(json_in, json_in_len)?;
            objectiveai_sdk::functions::alpha_vector::check::check_alpha_leaf_vector_function(
                &function, None,
            )
            .map_err(|e| e)?;
            Ok(Vec::new())
        })
    }
}

/// Alpha check for a branch vector function (depth > 0, vector output).
///
/// Input: Two JSON buffers — function definition, then optional children map.
/// Pass null/0 for children_in/children_in_len if no children.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_alpha_check_branch_vector_function(
    function_in: *const u8,
    function_in_len: usize,
    children_in: *const u8,
    children_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let function: objectiveai_sdk::functions::alpha_vector::RemoteFunction =
                from_json(function_in, function_in_len)?;
            let children: Option<
                std::collections::HashMap<String, objectiveai_sdk::functions::FullRemoteFunction>,
            > = if children_in.is_null() || children_in_len == 0 {
                None
            } else {
                Some(from_json(children_in, children_in_len)?)
            };
            objectiveai_sdk::functions::alpha_vector::check::check_alpha_branch_vector_function(
                &function,
                children.as_ref(),
                None,
            )
            .map_err(|e| e)?;
            Ok(Vec::new())
        })
    }
}

// ---------------------------------------------------------------------------
// Streaming Chunk Merging
// ---------------------------------------------------------------------------

/// Merges two AgentCompletionChunks via push and returns the merged result.
///
/// Input: Two JSON buffers — chunk A, then chunk B.
/// Output: JSON of the merged chunk.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_agent_completion_chunk_merged(
    a_in: *const u8,
    a_in_len: usize,
    b_in: *const u8,
    b_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut a: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
                from_json(a_in, a_in_len)?;
            let b: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
                from_json(b_in, b_in_len)?;
            a.push(&b);
            to_json(&a)
        })
    }
}

/// Merges two VectorCompletionChunks via push and returns the merged result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_vector_completion_chunk_merged(
    a_in: *const u8,
    a_in_len: usize,
    b_in: *const u8,
    b_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut a: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
                from_json(a_in, a_in_len)?;
            let b: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
                from_json(b_in, b_in_len)?;
            a.push(&b);
            to_json(&a)
        })
    }
}

/// Merges two FunctionExecutionChunks via push and returns the merged result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_function_execution_chunk_merged(
    a_in: *const u8,
    a_in_len: usize,
    b_in: *const u8,
    b_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut a: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
                from_json(a_in, a_in_len)?;
            let b: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
                from_json(b_in, b_in_len)?;
            a.push(&b);
            to_json(&a)
        })
    }
}

/// Merges two FunctionProfileComputationChunks via push and returns the merged result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_function_profile_computation_chunk_merged(
    a_in: *const u8,
    a_in_len: usize,
    b_in: *const u8,
    b_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut a: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
                from_json(a_in, a_in_len)?;
            let b: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
                from_json(b_in, b_in_len)?;
            a.push(&b);
            to_json(&a)
        })
    }
}

// ---------------------------------------------------------------------------
// Streaming Chunk Normalization
// ---------------------------------------------------------------------------

/// Normalizes an AgentCompletionChunk by round-tripping through serde.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_agent_completion_chunk_normalized(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let a: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
                from_json(json_in, json_in_len)?;
            to_json(&a)
        })
    }
}

/// Normalizes a VectorCompletionChunk by round-tripping through serde.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_vector_completion_chunk_normalized(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let a: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
                from_json(json_in, json_in_len)?;
            to_json(&a)
        })
    }
}

/// Normalizes a FunctionExecutionChunk by round-tripping through serde.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_function_execution_chunk_normalized(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let a: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
                from_json(json_in, json_in_len)?;
            to_json(&a)
        })
    }
}

/// Normalizes a FunctionProfileComputationChunk by round-tripping through serde.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_function_profile_computation_chunk_normalized(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let a: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
                from_json(json_in, json_in_len)?;
            to_json(&a)
        })
    }
}

// ---------------------------------------------------------------------------
// Streaming Chunk to Unary Conversion
// ---------------------------------------------------------------------------

/// Converts an accumulated AgentCompletionChunk to an AgentCompletion (unary).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_agent_completion_chunk_to_unary(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let a: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk =
                from_json(json_in, json_in_len)?;
            let unary: objectiveai_sdk::agent::completions::response::unary::AgentCompletion = a.into();
            to_json(&unary)
        })
    }
}

/// Converts an accumulated VectorCompletionChunk to a VectorCompletion (unary).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_vector_completion_chunk_to_unary(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let a: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk =
                from_json(json_in, json_in_len)?;
            let unary: objectiveai_sdk::vector::completions::response::unary::VectorCompletion =
                a.into();
            to_json(&unary)
        })
    }
}

/// Converts an accumulated FunctionExecutionChunk to a FunctionExecution (unary).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_function_execution_chunk_to_unary(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let a: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk =
                from_json(json_in, json_in_len)?;
            let unary: objectiveai_sdk::functions::executions::response::unary::FunctionExecution =
                a.into();
            to_json(&unary)
        })
    }
}

/// Converts an accumulated FunctionProfileComputationChunk to a FunctionProfileComputation (unary).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_function_profile_computation_chunk_to_unary(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let a: objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk =
                from_json(json_in, json_in_len)?;
            let unary: objectiveai_sdk::functions::profiles::computations::response::unary::FunctionProfileComputation =
                a.into();
            to_json(&unary)
        })
    }
}

// ---------------------------------------------------------------------------
// Normalize for tests
// ---------------------------------------------------------------------------

/// Normalizes an AgentCompletion for test snapshot stability.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_normalize_agent_completion_for_tests(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut a: objectiveai_sdk::agent::completions::response::unary::AgentCompletion =
                from_json(json_in, json_in_len)?;
            a.normalize_for_tests();
            to_json(&a)
        })
    }
}

/// Normalizes a VectorCompletion for test snapshot stability.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_normalize_vector_completion_for_tests(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut a: objectiveai_sdk::vector::completions::response::unary::VectorCompletion =
                from_json(json_in, json_in_len)?;
            a.normalize_for_tests();
            to_json(&a)
        })
    }
}

/// Normalizes a FunctionExecution for test snapshot stability.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_normalize_function_execution_for_tests(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut a: objectiveai_sdk::functions::executions::response::unary::FunctionExecution =
                from_json(json_in, json_in_len)?;
            a.normalize_for_tests();
            to_json(&a)
        })
    }
}

/// Normalizes a FunctionProfileComputation for test snapshot stability.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_normalize_function_profile_computation_for_tests(
    json_in: *const u8,
    json_in_len: usize,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let mut a: objectiveai_sdk::functions::profiles::computations::response::unary::FunctionProfileComputation =
                from_json(json_in, json_in_len)?;
            a.normalize_for_tests();
            to_json(&a)
        })
    }
}

// ---------------------------------------------------------------------------
// Seed → bytes helper
// ---------------------------------------------------------------------------

fn seed_to_bytes(has_seed: i32, seed: i64) -> Vec<u8> {
    use rand::prelude::*;

    let seed_val: u64 = if has_seed != 0 {
        seed as u64
    } else {
        rand::random()
    };

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed_val);
    let mut bytes = vec![0u8; 4096];
    rng.fill_bytes(&mut bytes);
    bytes
}

// ---------------------------------------------------------------------------
// Generate arbitrary chunks
// ---------------------------------------------------------------------------

/// Generates a random AgentCompletionChunk from a seed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_generate_agent_completion_chunk(
    has_seed: i32,
    seed: i64,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let bytes = seed_to_bytes(has_seed, seed);
            let mut u = arbitrary::Unstructured::new(&bytes);
            let chunk = objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk::arbitrary(&mut u)
                .map_err(|e| e.to_string())?;
            to_json(&chunk)
        })
    }
}

/// Generates a random VectorCompletionChunk from a seed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_generate_vector_completion_chunk(
    has_seed: i32,
    seed: i64,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let bytes = seed_to_bytes(has_seed, seed);
            let mut u = arbitrary::Unstructured::new(&bytes);
            let chunk = objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk::arbitrary(&mut u)
                .map_err(|e| e.to_string())?;
            to_json(&chunk)
        })
    }
}

/// Generates a random FunctionExecutionChunk from a seed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_generate_function_execution_chunk(
    has_seed: i32,
    seed: i64,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let bytes = seed_to_bytes(has_seed, seed);
            let mut u = arbitrary::Unstructured::new(&bytes);
            let chunk = objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk::arbitrary(&mut u)
                .map_err(|e| e.to_string())?;
            to_json(&chunk)
        })
    }
}

/// Generates a random FunctionProfileComputationChunk from a seed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn objectiveai_generate_function_profile_computation_chunk(
    has_seed: i32,
    seed: i64,
    json_out: *mut *mut u8,
    json_out_len: *mut usize,
) -> i32 {
    unsafe {
        run(json_out, json_out_len, || {
            let bytes = seed_to_bytes(has_seed, seed);
            let mut u = arbitrary::Unstructured::new(&bytes);
            let chunk = objectiveai_sdk::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk::arbitrary(&mut u)
                .map_err(|e| e.to_string())?;
            to_json(&chunk)
        })
    }
}
