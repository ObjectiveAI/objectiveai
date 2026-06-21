//! Per-input validation of compiled tasks and output expressions.

use crate::agent::completions::message::{Message, RichContent};
use crate::functions::expression::{
    Params, ParamsRef, TaskOutput, TaskOutputOwned,
};
use crate::functions::{
    CompiledTask, Function, RemoteFunction, Task, VectorCompletionTask,
};
use rand::Rng;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

/// Number of randomized output expression evaluations to verify variance.
const OUTPUT_EXPRESSION_TRIALS: usize = 100;

/// Whether the parent function is scalar or vector (with a known output_length).
enum FunctionType {
    Scalar,
    Vector { output_length: u64 },
}

/// Validates a single input: compiles tasks, checks all constraints, and
/// returns the compiled tasks for further use (e.g. diversity tracking).
pub(crate) fn compile_and_validate_one_input(
    input_label: &str,
    function: &RemoteFunction,
    input: &crate::functions::expression::InputValue,
    children: Option<&HashMap<String, RemoteFunction>>,
) -> Result<Vec<Option<CompiledTask>>, String> {
    // Determine parent function type (scalar or vector with output_length)
    let function_type = match function {
        RemoteFunction::Scalar { .. } => FunctionType::Scalar,
        RemoteFunction::Vector { output_length, .. } => {
            let params = Params::Ref(ParamsRef {
                input,
                output: None,
                map: None,
            });
            let len =
                output_length.clone().compile_one(&params).map_err(|e| {
                    format!(
                        "CV02: Input {}: output_length compilation failed: {}",
                        input_label, e
                    )
                })?;
            FunctionType::Vector { output_length: len }
        }
    };

    // compile_tasks takes self by value, so we clone into a Function
    let func = Function::Remote(function.clone());
    let compiled_tasks = func.compile_tasks(input).map_err(|e| {
        format!(
            "CV03: Input {}: task compilation failed: {}\n\nInput: {}",
            input_label,
            e,
            serde_json::to_string(input).unwrap_or_default()
        )
    })?;

    // At least one task must not be skipped
    if compiled_tasks.iter().all(|t| t.is_none()) {
        return Err(format!(
            "CV42: Input {}: all tasks were skipped — at least one task must run for every valid input",
            input_label
        ));
    }

    // Validate each compiled task
    for (j, compiled_task) in compiled_tasks.iter().enumerate() {
        let compiled_task = match compiled_task {
            Some(ct) => ct,
            None => continue, // skipped task
        };

        match compiled_task {
            CompiledTask::One(task) => {
                validate_compiled_task(input_label, j, None, task, children)?;
                validate_output_expression(
                    input_label,
                    j,
                    input,
                    compiled_task,
                    task,
                    &function_type,
                    children,
                )?;
            }
            CompiledTask::Many(tasks) => {
                for (k, task) in tasks.iter().enumerate() {
                    validate_compiled_task(
                        input_label,
                        j,
                        Some(k),
                        task,
                        children,
                    )?;
                }
                // Validate the mapped output expression using the first
                // task as representative (all share the same output expr)
                if let Some(first) = tasks.first() {
                    validate_output_expression(
                        input_label,
                        j,
                        input,
                        compiled_task,
                        first,
                        &function_type,
                        children,
                    )?;
                }
            }
        }
    }

    Ok(compiled_tasks)
}

/// Validates a single compiled task's inputs.
fn validate_compiled_task(
    input_label: &str,
    task_index: usize,
    map_index: Option<usize>,
    task: &Task,
    children: Option<&HashMap<String, RemoteFunction>>,
) -> Result<(), String> {
    let location = match map_index {
        Some(k) => {
            format!("Input {}, task [{}][{}]", input_label, task_index, k)
        }
        None => format!("Input {}, task [{}]", input_label, task_index),
    };

    match task {
        Task::PlaceholderScalarFunction(t) => {
            if !t.input_schema.validate_input(&t.input) {
                return Err(format!(
                    "CV04: {}: compiled input does not match placeholder's input_schema\n\nInput: {}\n\nSchema: {}",
                    location,
                    serde_json::to_string(&t.input).unwrap_or_default(),
                    serde_json::to_string(&t.input_schema).unwrap_or_default(),
                ));
            }
        }
        Task::PlaceholderVectorFunction(t) => {
            if !t.input_schema.validate_input(&t.input) {
                return Err(format!(
                    "CV05: {}: compiled input does not match placeholder's input_schema\n\nInput: {}\n\nSchema: {}",
                    location,
                    serde_json::to_string(&t.input).unwrap_or_default(),
                    serde_json::to_string(&t.input_schema).unwrap_or_default(),
                ));
            }
        }
        Task::VectorCompletion(vc) => {
            check_compiled_vector_completion(&location, vc)?;
        }
        Task::ScalarFunction(t) => {
            if let Some(children) = children {
                let key = t.path.key();
                let child = children.get(&key).ok_or_else(|| {
                    format!(
                        "CV06: {}: referenced scalar.function '{}' not found in children",
                        location, key
                    )
                })?;
                if !child.input_schema().validate_input(&t.input) {
                    return Err(format!(
                        "CV07: {}: compiled input does not match child function's input_schema ({})\n\nInput: {}\n\nSchema: {}",
                        location,
                        key,
                        serde_json::to_string(&t.input).unwrap_or_default(),
                        serde_json::to_string(child.input_schema())
                            .unwrap_or_default(),
                    ));
                }
            }
        }
        Task::VectorFunction(t) => {
            if let Some(children) = children {
                let key = t.path.key();
                let child = children.get(&key).ok_or_else(|| {
                    format!(
                        "CV08: {}: referenced vector.function '{}' not found in children",
                        location, key
                    )
                })?;
                if !child.input_schema().validate_input(&t.input) {
                    return Err(format!(
                        "CV09: {}: compiled input does not match child function's input_schema ({})\n\nInput: {}\n\nSchema: {}",
                        location,
                        key,
                        serde_json::to_string(&t.input).unwrap_or_default(),
                        serde_json::to_string(child.input_schema())
                            .unwrap_or_default(),
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Generates randomized mock raw outputs, evaluates the output expression
/// multiple times, validates each result against the parent function type,
/// and checks that all results are distinct (ensuring the expression actually
/// derives its output from the raw result, not returning a fixed value).
fn validate_output_expression(
    input_label: &str,
    task_index: usize,
    input: &crate::functions::expression::InputValue,
    compiled_task: &CompiledTask,
    representative_task: &Task,
    function_type: &FunctionType,
    children: Option<&HashMap<String, RemoteFunction>>,
) -> Result<(), String> {
    let location = format!("Input {}, task [{}]", input_label, task_index);

    // Determine the output shape info we need for random generation
    // (returns None if we can't construct mocks, e.g. vector.function without children)
    let shape = match compiled_task {
        CompiledTask::One(task) => {
            task_output_shape(task, children, &location)?
        }
        CompiledTask::Many(tasks) => {
            mapped_task_output_shape(tasks, children, &location)?
        }
    };

    let Some(shape) = shape else {
        return Ok(());
    };

    let mut rng = rand::rng();
    let mut seen = HashSet::new();

    for trial in 0..OUTPUT_EXPRESSION_TRIALS {
        let mock_output = random_task_output(&shape, &mut rng);

        let result = representative_task
            .compile_output(input, mock_output)
            .map_err(|e| {
                format!(
                    "CV10: {}: output expression evaluation failed (trial {}): {}",
                    location, trial, e
                )
            })?;

        // Validate against parent function type
        validate_function_output(&location, function_type, &result)?;

        // Check uniqueness — serialize to string for comparison
        let key = serde_json::to_string(&result).unwrap_or_default();
        if !seen.insert(key) {
            return Err(format!(
                "CV11: {}: output expression produced duplicate results across \
                 {} randomized trials — the expression must derive its \
                 output from the raw task result, not return a fixed value",
                location,
                trial + 1,
            ));
        }
    }

    Ok(())
}

/// Validates a `TaskOutputOwned` against the expected parent function type.
fn validate_function_output(
    location: &str,
    function_type: &FunctionType,
    result: &TaskOutputOwned,
) -> Result<(), String> {
    match (function_type, result) {
        (FunctionType::Scalar, TaskOutputOwned::Scalar(s)) => {
            if *s < Decimal::new(-1, 2) || *s > Decimal::new(101, 2) {
                return Err(format!(
                    "CV12: {}: output expression produced scalar {} which is outside \
                     the valid range [-0.01, 1.01]",
                    location, s
                ));
            }
        }
        (FunctionType::Scalar, TaskOutputOwned::Vector(v)) => {
            return Err(format!(
                "CV13: {}: output expression produced a vector of length {} but \
                 parent is a scalar function (expected a scalar value)",
                location,
                v.len()
            ));
        }
        (
            FunctionType::Vector { output_length },
            TaskOutputOwned::Vector(v),
        ) => {
            if v.len() as u64 != *output_length {
                return Err(format!(
                    "CV14: {}: output expression produced a vector of length {} but \
                     parent function's output_length is {}",
                    location,
                    v.len(),
                    output_length
                ));
            }
            let sum: Decimal = v.iter().copied().sum();
            if sum < Decimal::new(99, 2) || sum > Decimal::new(101, 2) {
                return Err(format!(
                    "CV15: {}: output expression produced a vector summing to {} \
                     which is outside the valid range [0.99, 1.01]",
                    location, sum
                ));
            }
        }
        (FunctionType::Vector { .. }, TaskOutputOwned::Scalar(s)) => {
            return Err(format!(
                "CV16: {}: output expression produced scalar {} but parent is a \
                 vector function (expected a vector)",
                location, s
            ));
        }
        (_, TaskOutputOwned::Err { error }) => {
            return Err(format!(
                "CV17: {}: output expression produced an error: {}",
                location,
                serde_json::to_string(error).unwrap_or_default()
            ));
        }
        (_, TaskOutputOwned::Vectors(vecs)) => {
            return Err(format!(
                "CV17: {}: output expression produced Vectors({} sub-vectors) \
                 which is not valid as a final function output",
                location,
                vecs.len()
            ));
        }
    }
    Ok(())
}

// --- Output shape descriptors (what random output to generate) ---

/// Describes the shape of a task's raw output for random generation.
enum OutputShape {
    /// Vector completion with `n` responses.
    VectorCompletion(usize),
    /// Scalar function output (single value in [0, 1]).
    Scalar,
    /// Vector function output with `n` elements.
    Vector(u64),
    /// Mapped vector completion — one VectorCompletion per task instance.
    MapVectorCompletion(Vec<usize>),
    /// Mapped scalar function — one scalar per task instance.
    MapScalar(usize),
    /// Mapped vector function — one vector per task instance.
    MapVector(Vec<u64>),
}

/// Determines the output shape for a single (non-mapped) compiled task.
/// Returns `None` if shape can't be determined (e.g., vector.function without children).
fn task_output_shape(
    task: &Task,
    children: Option<&HashMap<String, RemoteFunction>>,
    location: &str,
) -> Result<Option<OutputShape>, String> {
    match task {
        Task::VectorCompletion(vc) => {
            Ok(Some(OutputShape::VectorCompletion(vc.responses.len())))
        }
        Task::ScalarFunction(_) | Task::PlaceholderScalarFunction(_) => {
            Ok(Some(OutputShape::Scalar))
        }
        Task::VectorFunction(t) => {
            let Some(n) = resolve_vector_function_output_length(
                &t.path, &t.input, children, location,
            )?
            else {
                return Ok(None);
            };
            Ok(Some(OutputShape::Vector(n)))
        }
        Task::PlaceholderVectorFunction(t) => {
            let params = Params::Ref(ParamsRef {
                input: &t.input,
                output: None,
                map: None,
            });
            let n =
                t.output_length.clone().compile_one(&params).map_err(|e| {
                    format!(
                        "CV18: {}: placeholder vector function output_length \
                         compilation failed: {}",
                        location, e
                    )
                })?;
            Ok(Some(OutputShape::Vector(n)))
        }
    }
}

/// Determines the output shape for a mapped (Many) compiled task.
fn mapped_task_output_shape(
    tasks: &[Task],
    children: Option<&HashMap<String, RemoteFunction>>,
    location: &str,
) -> Result<Option<OutputShape>, String> {
    if tasks.is_empty() {
        return Err(format!(
            "CV19: {}: mapped task has no instances",
            location
        ));
    }

    match &tasks[0] {
        Task::VectorCompletion(_) => {
            let sizes: Vec<usize> = tasks
                .iter()
                .map(|task| match task {
                    Task::VectorCompletion(vc) => Ok(vc.responses.len()),
                    _ => Err(format!(
                        "CV20: {}: mixed task types in mapped task",
                        location
                    )),
                })
                .collect::<Result<_, _>>()?;
            Ok(Some(OutputShape::MapVectorCompletion(sizes)))
        }
        Task::ScalarFunction(_) | Task::PlaceholderScalarFunction(_) => {
            Ok(Some(OutputShape::MapScalar(tasks.len())))
        }
        Task::VectorFunction(_) => {
            let mut lengths = Vec::with_capacity(tasks.len());
            for task in tasks {
                match task {
                    Task::VectorFunction(t) => {
                        let Some(n) = resolve_vector_function_output_length(
                            &t.path, &t.input, children, location,
                        )?
                        else {
                            return Ok(None);
                        };
                        lengths.push(n);
                    }
                    _ => {
                        return Err(format!(
                            "CV21: {}: mixed task types in mapped task",
                            location
                        ));
                    }
                }
            }
            Ok(Some(OutputShape::MapVector(lengths)))
        }
        Task::PlaceholderVectorFunction(_) => {
            let lengths: Vec<u64> = tasks
                .iter()
                .map(|task| match task {
                    Task::PlaceholderVectorFunction(t) => {
                        let params = Params::Ref(ParamsRef {
                            input: &t.input,
                            output: None,
                            map: None,
                        });
                        t.output_length.clone().compile_one(&params).map_err(
                            |e| {
                                format!(
                                    "CV22: {}: placeholder vector output_length \
                                     compilation failed: {}",
                                    location, e
                                )
                            },
                        )
                    }
                    _ => Err(format!(
                        "CV23: {}: mixed task types in mapped task",
                        location
                    )),
                })
                .collect::<Result<_, _>>()?;
            Ok(Some(OutputShape::MapVector(lengths)))
        }
    }
}

// --- Random output generation ---

/// Creates a randomized `TaskOutput` from an `OutputShape`.
fn random_task_output<'a>(
    shape: &OutputShape,
    rng: &mut impl Rng,
) -> TaskOutput<'a> {
    match shape {
        OutputShape::VectorCompletion(n) => {
            TaskOutput::Owned(TaskOutputOwned::Vector(random_scores(*n, rng)))
        }
        OutputShape::Scalar => {
            let v: f64 = rng.random_range(0.01..0.99);
            TaskOutput::Owned(TaskOutputOwned::Scalar(
                Decimal::from_f64_retain(v).unwrap_or(Decimal::new(5, 1)),
            ))
        }
        OutputShape::Vector(n) => TaskOutput::Owned(TaskOutputOwned::Vector(
            random_scores(*n as usize, rng),
        )),
        OutputShape::MapVectorCompletion(sizes) => {
            let outputs: Vec<Vec<Decimal>> =
                sizes.iter().map(|&n| random_scores(n, rng)).collect();
            TaskOutput::Owned(TaskOutputOwned::Vectors(outputs))
        }
        OutputShape::MapScalar(count) => {
            let scalars: Vec<Decimal> = (0..*count)
                .map(|_| {
                    let v: f64 = rng.random_range(0.01..0.99);
                    Decimal::from_f64_retain(v).unwrap_or(Decimal::new(5, 1))
                })
                .collect();
            TaskOutput::Owned(TaskOutputOwned::Vector(scalars))
        }
        OutputShape::MapVector(lengths) => {
            let outputs: Vec<Vec<Decimal>> = lengths
                .iter()
                .map(|&n| random_scores(n as usize, rng))
                .collect();
            TaskOutput::Owned(TaskOutputOwned::Vectors(outputs))
        }
    }
}

/// Generates a random vector of `n` non-negative Decimals that sum to ~1.
fn random_scores(n: usize, rng: &mut impl Rng) -> Vec<Decimal> {
    if n == 0 {
        return vec![];
    }
    // Generate random f64 values, normalize to sum to 1
    let raw: Vec<f64> =
        (0..n).map(|_| rng.random_range(0.01_f64..1.0)).collect();
    let sum: f64 = raw.iter().sum();
    raw.iter()
        .map(|&v| Decimal::from_f64_retain(v / sum).unwrap_or(Decimal::ZERO))
        .collect()
}

/// Resolves the output_length for a vector.function task by looking up the
/// child function and compiling its output_length expression with the task input.
/// Returns `None` if children is `None` (output validation is skipped).
fn resolve_vector_function_output_length(
    path: &crate::RemotePath,
    task_input: &crate::functions::expression::InputValue,
    children: Option<&HashMap<String, RemoteFunction>>,
    location: &str,
) -> Result<Option<u64>, String> {
    let key = path.key();
    let Some(children) = children else {
        return Ok(None); // skip output validation without children
    };
    let child = children.get(&key).ok_or_else(|| {
        format!(
            "CV24: {}: referenced vector.function '{}' not found in children",
            location, key
        )
    })?;
    let output_length_expr = child.output_length().ok_or_else(|| {
        format!(
            "CV25: {}: child function '{}' is not a vector function",
            location, key
        )
    })?;
    let params = Params::Ref(ParamsRef {
        input: task_input,
        output: None,
        map: None,
    });
    let n = output_length_expr
        .clone()
        .compile_one(&params)
        .map_err(|e| {
            format!(
                "CV26: {}: child function '{}' output_length compilation failed: {}",
                location, key, e
            )
        })?;
    Ok(Some(n))
}

/// Validates a compiled vector completion task:
/// - At least 1 message
/// - Message content is content parts, not plain strings
/// - At least 2 responses
/// - Response content is content parts, not plain strings
fn check_compiled_vector_completion(
    location: &str,
    vc: &VectorCompletionTask,
) -> Result<(), String> {
    // At least 1 message
    if vc.messages.is_empty() {
        return Err(format!(
            "CV27: {}: compiled task must have at least 1 message",
            location
        ));
    }

    // Message content must be content parts
    for (j, msg) in vc.messages.iter().enumerate() {
        check_compiled_message_content(location, j, msg)?;
    }

    // At least 2 responses
    if vc.responses.len() < 2 {
        return Err(format!(
            "CV28: {}: compiled task must have at least 2 responses, found {}. Try setting `minItems` to 2 on the `input_schema`.",
            location,
            vc.responses.len()
        ));
    }

    // Response content must be content parts
    for (j, resp) in vc.responses.iter().enumerate() {
        if matches!(resp, RichContent::Text(_)) {
            return Err(format!(
                "CV29: {}, response [{}]: compiled response must be an array of content parts, \
                 not a plain string",
                location, j
            ));
        }
    }

    Ok(())
}

/// Extracts the serialized input from a compiled function/placeholder task.
/// Returns empty string for vector.completion tasks (not applicable).
pub(crate) fn extract_task_input(task: &Task) -> String {
    match extract_task_input_value(task) {
        Some(input) => serde_json::to_string(input).unwrap_or_default(),
        None => String::new(),
    }
}

/// Returns a reference to the compiled input of a function/placeholder task.
pub(crate) fn extract_task_input_value(
    task: &Task,
) -> Option<&crate::functions::expression::InputValue> {
    match task {
        Task::ScalarFunction(t) => Some(&t.input),
        Task::VectorFunction(t) => Some(&t.input),
        Task::PlaceholderScalarFunction(t) => Some(&t.input),
        Task::PlaceholderVectorFunction(t) => Some(&t.input),
        Task::VectorCompletion(_) => None,
    }
}

/// Checks that a compiled message's content is parts, not a plain string.
fn check_compiled_message_content(
    location: &str,
    msg_index: usize,
    msg: &Message,
) -> Result<(), String> {
    match msg {
        Message::User(user) => {
            if matches!(user.content, RichContent::Text(_)) {
                return Err(format!(
                    "CV39: {}, message [{}] (user): compiled content must be an array of \
                     content parts, not a plain string",
                    location, msg_index
                ));
            }
        }
        Message::Assistant(asst) => {
            if let Some(content) = &asst.content {
                if matches!(content, RichContent::Text(_)) {
                    return Err(format!(
                        "CV40: {}, message [{}] (assistant): compiled content must be an array of \
                         content parts, not a plain string",
                        location, msg_index
                    ));
                }
            }
        }
        Message::Tool(tool) => {
            if matches!(tool.content, RichContent::Text(_)) {
                return Err(format!(
                    "CV41: {}, message [{}] (tool): compiled content must be an array of \
                     content parts, not a plain string",
                    location, msg_index
                ));
            }
        }
    }
    Ok(())
}
