//! Function execution client.

use crate::{
    ctx, functions,
    util::{ChoiceIndexer, StreamOnce},
    vector,
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hasher, sync::Arc, time};

/// Generates a unique response ID for Function executions.
pub fn response_id(created: u64) -> String {
    crate::util::response_id(Some("fnexec"), created)
}

/// Inverts a single `TaskOutputOwned` in place.
///
/// - `Scalar(x)` → `Scalar(1 - x)` (distance-from-1 becomes distance-from-0).
/// - `Vector(v)` → rank-inverted: the position that had the highest value
///   ends up with the lowest, the position that had the lowest ends up with
///   the highest, and so on. e.g. `[0.5, 0.2, 0.3]` → `[0.2, 0.5, 0.3]`.
///   Total sum is preserved (still a valid probability distribution).
/// - `Vectors(vv)` → each inner vector rank-inverted.
/// - `Err { .. }` is left untouched — there is no meaningful inverse of an error.
fn invert_task_output(output: &mut objectiveai_sdk::functions::expression::TaskOutputOwned) {
    use objectiveai_sdk::functions::expression::TaskOutputOwned;
    match output {
        TaskOutputOwned::Scalar(d) => {
            *d = rust_decimal::Decimal::ONE - *d;
        }
        TaskOutputOwned::Vector(v) => invert_vector_in_place(v),
        TaskOutputOwned::Vectors(vv) => {
            for v in vv.iter_mut() {
                invert_vector_in_place(v);
            }
        }
        TaskOutputOwned::Err { .. } => {}
    }
}

/// Rank-inverts a vector of decimals in place: the position that ranked
/// highest by value receives the smallest value, and so on.
///
/// Stable on ties — positions whose original values are equal keep their
/// relative order, so `[0.4, 0.4, 0.2]` → `[0.2, 0.4, 0.4]` (deterministic).
fn invert_vector_in_place(v: &mut Vec<rust_decimal::Decimal>) {
    if v.len() <= 1 {
        return;
    }
    // Sort original indices by value descending (stable; ties keep input order).
    let mut indexed: Vec<(usize, rust_decimal::Decimal)> =
        v.iter().enumerate().map(|(i, x)| (i, *x)).collect();
    indexed.sort_by(|a, b| b.1.cmp(&a.1));
    // Sort the values themselves ascending.
    let mut sorted_asc: Vec<rust_decimal::Decimal> = v.clone();
    sorted_asc.sort();
    // Highest-rank position gets the smallest value, etc.
    for (rank, (orig_idx, _)) in indexed.into_iter().enumerate() {
        v[orig_idx] = sorted_asc[rank];
    }
}

/// Recursively inverts every `output` field in a `FunctionExecutionChunk`,
/// including those inside nested function-execution task chunks.
///
/// VectorCompletion task chunks carry raw vote/score data, not a function
/// "output", so their inner `scores`/`votes` are intentionally untouched —
/// `invert` is a final-output transformation, not a re-scoring of the
/// underlying votes.
fn invert_function_execution_chunk(
    chunk: &mut objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk,
) {
    if let Some(output) = chunk.output.as_mut() {
        invert_task_output(&mut output.output);
    }
    for task in chunk.tasks.iter_mut() {
        if let objectiveai_sdk::functions::executions::response::streaming::TaskChunk::FunctionExecution(ft) = task {
            invert_function_execution_chunk(&mut ft.inner);
        }
    }
}

/// Computes the final function output as a weighted average of task outputs.
///
/// All task outputs are already validated `TaskOutputOwned` (scalar or vector)
/// by their respective output expressions. This function is deterministically
/// infallible - all inputs are assumed valid.
///
/// The weights are L1-normalized for the indices that are present (non-None, non-error).
fn compute_weighted_function_output(
    function_type: &functions::FunctionType,
    profile_weights: &[rust_decimal::Decimal],
    task_outputs: &[Option<
        objectiveai_sdk::functions::expression::TaskOutputOwned,
    >],
) -> objectiveai_sdk::functions::expression::TaskOutputOwned {
    use objectiveai_sdk::functions::expression::TaskOutputOwned;
    use rust_decimal::Decimal;

    // Collect (weight, TaskOutputOwned) pairs from present task outputs
    let mut weighted_outputs: Vec<(Decimal, &TaskOutputOwned)> = Vec::new();
    let mut total_weight = Decimal::ZERO;

    for (i, task_output) in task_outputs.iter().enumerate() {
        let weight = profile_weights.get(i).copied().unwrap_or(Decimal::ZERO);
        if weight == Decimal::ZERO {
            continue;
        }

        let fn_output = match task_output {
            Some(output) => output,
            None => continue,
        };

        // Skip error outputs (these shouldn't be here, but just in case)
        if matches!(fn_output, TaskOutputOwned::Err { .. }) {
            continue;
        }

        total_weight += weight;
        weighted_outputs.push((weight, fn_output));
    }

    // If no valid outputs, return error (shouldn't happen if caller filters properly)
    if weighted_outputs.is_empty() || total_weight == Decimal::ZERO {
        return TaskOutputOwned::Err {
            error: serde_json::Value::Null,
        };
    }

    // Compute weighted average with L1-normalized weights
    match function_type {
        functions::FunctionType::Scalar => {
            let mut weighted_sum = Decimal::ZERO;
            for (weight, fn_output) in &weighted_outputs {
                match fn_output {
                    TaskOutputOwned::Scalar(s) => {
                        // L1-normalize: weight / total_weight
                        weighted_sum += (*weight / total_weight) * s;
                    }
                    _ => {
                        panic!(
                            "expected scalar output in scalar function, got {:?}",
                            fn_output
                        );
                    }
                }
            }
            TaskOutputOwned::Scalar(weighted_sum)
        }
        functions::FunctionType::Vector { .. } => {
            // Get vector length from first output
            let vec_len = weighted_outputs
                .iter()
                .find_map(|(_, o)| match o {
                    TaskOutputOwned::Vector(v) => Some(v.len()),
                    _ => None,
                })
                .expect("expected at least one vector output");

            // Compute weighted average for each element with L1-normalized weights
            let mut result = vec![Decimal::ZERO; vec_len];
            for (weight, fn_output) in &weighted_outputs {
                match fn_output {
                    TaskOutputOwned::Vector(v) => {
                        if v.len() != vec_len {
                            panic!(
                                "vector length mismatch: expected {}, got {}",
                                vec_len,
                                v.len()
                            );
                        }
                        let normalized_weight = *weight / total_weight;
                        for (j, val) in v.iter().enumerate() {
                            result[j] += normalized_weight * val;
                        }
                    }
                    _ => {
                        panic!(
                            "expected vector output in vector function, got {:?}",
                            fn_output
                        );
                    }
                }
            }
            TaskOutputOwned::Vector(result)
        }
    }
}
/// Applies a task's output expression to transform a raw task output into a TaskOutputOwned.
///
/// The expression receives `output` which is one of 4 variants:
/// - `Function(TaskOutputOwned)` - single function task result
/// - `MapFunction(Vec<TaskOutputOwned>)` - mapped function task results
/// - `VectorCompletion(VectorCompletionOutput)` - single vector completion result
/// - `MapVectorCompletion(Vec<VectorCompletionOutput>)` - mapped vector completion results
///
/// The expression transforms this into a `TaskOutputOwned`. The output is validated against
/// the function type (scalar vs vector) and optional output length.
///
/// Returns the output (possibly as `TaskOutputOwned::Err` if invalid) and an optional error.
fn apply_task_output_expression(
    input: &objectiveai_sdk::functions::expression::InputValue,
    task_output: objectiveai_sdk::functions::expression::TaskOutputOwned,
    output_expression: &objectiveai_sdk::functions::expression::Expression,
    invert_output: bool,
    function_type: &functions::FunctionType,
) -> (
    objectiveai_sdk::functions::expression::TaskOutputOwned,
    Option<objectiveai_sdk::error::ResponseError>,
) {
    use objectiveai_sdk::functions::expression::{
        TaskOutputOwned, Params, ParamsRef, TaskOutput,
    };
    use rust_decimal::Decimal;

    fn invert_function_output(output: TaskOutputOwned) -> TaskOutputOwned {
        match output {
            TaskOutputOwned::Scalar(s) => {
                TaskOutputOwned::Scalar(Decimal::ONE - s)
            }
            TaskOutputOwned::Vector(mut v) => {
                if v.is_empty() {
                    return TaskOutputOwned::Vector(v);
                }
                for x in &mut v {
                    *x = Decimal::ONE - *x;
                }
                let sum: Decimal = v.iter().map(|x| x.abs()).sum();
                if sum == Decimal::ZERO {
                    let uniform = Decimal::ONE / Decimal::from(v.len());
                    for x in &mut v {
                        *x = uniform;
                    }
                } else {
                    for x in &mut v {
                        *x /= sum;
                    }
                }
                TaskOutputOwned::Vector(v)
            }
            TaskOutputOwned::Vectors(vecs) => {
                TaskOutputOwned::Vectors(
                    vecs.into_iter()
                        .map(|v| match invert_function_output(TaskOutputOwned::Vector(v)) {
                            TaskOutputOwned::Vector(v) => v,
                            _ => unreachable!(),
                        })
                        .collect(),
                )
            }
            TaskOutputOwned::Err { error } => TaskOutputOwned::Err { error },
        }
    }

    // Build params with input and the task output (one of 4 variants)
    let params = Params::Ref(ParamsRef {
        input,
        output: Some(TaskOutput::Owned(task_output)),
        map: None,
    });

    // Evaluate the expression - it transforms the raw output into TaskOutputOwned
    let result = match output_expression.compile_one::<TaskOutputOwned>(&params)
    {
        Ok(result) => result,
        Err(e) => {
            return (
                TaskOutputOwned::Err {
                    error: serde_json::Value::Null,
                },
                Some(objectiveai_sdk::error::ResponseError::from(
                    &super::Error::InvalidAppExpression(e),
                )),
            );
        }
    };

    // Validate the output against the function type
    let (validated, err) = match (function_type, result) {
        // Scalar function must return scalar output (allow -0.01 to 1.01 for floating point tolerance)
        (functions::FunctionType::Scalar, TaskOutputOwned::Scalar(s)) => {
            if s >= rust_decimal::dec!(-0.01) && s <= rust_decimal::dec!(1.01) {
                (TaskOutputOwned::Scalar(s), None)
            } else {
                (
                    TaskOutputOwned::Scalar(s).into_err(),
                    Some(objectiveai_sdk::error::ResponseError::from(
                        &super::Error::InvalidScalarOutput,
                    )),
                )
            }
        }
        // Scalar function got vector output - error
        (
            functions::FunctionType::Scalar,
            result @ TaskOutputOwned::Vector(_),
        ) => (
            result.into_err(),
            Some(objectiveai_sdk::error::ResponseError::from(
                &super::Error::InvalidScalarOutput,
            )),
        ),
        // Vector function must return vector output
        (
            functions::FunctionType::Vector { output_length, .. },
            TaskOutputOwned::Vector(v),
        ) => {
            let sum: Decimal = v.iter().cloned().sum();
            let len_ok = output_length.is_none_or(|len| len == v.len() as u64);
            let sum_ok = sum >= rust_decimal::dec!(0.99)
                && sum <= rust_decimal::dec!(1.01);
            if len_ok && sum_ok {
                (TaskOutputOwned::Vector(v), None)
            } else {
                let err_len = output_length.unwrap_or(v.len() as u64) as usize;
                (
                    TaskOutputOwned::Vector(v).into_err(),
                    Some(objectiveai_sdk::error::ResponseError::from(
                        &super::Error::InvalidVectorOutput(err_len),
                    )),
                )
            }
        }
        // Vector function got scalar output - error
        (
            functions::FunctionType::Vector { output_length, .. },
            result @ TaskOutputOwned::Scalar(_),
        ) => (
            result.into_err(),
            Some(objectiveai_sdk::error::ResponseError::from(
                &super::Error::InvalidVectorOutput(
                    output_length.unwrap_or_default() as usize,
                ),
            )),
        ),
        // Vectors output is not expected from task output expressions
        (_, result @ TaskOutputOwned::Vectors(_)) => (
            result.into_err(),
            Some(objectiveai_sdk::error::ResponseError::from(
                &super::Error::InvalidScalarOutput,
            )),
        ),
        // Error output passes through - this means the expression itself produced an error value
        (_, TaskOutputOwned::Err { error: err_val }) => (
            TaskOutputOwned::Err {
                error: err_val.clone(),
            },
            Some(objectiveai_sdk::error::ResponseError {
                code: 400,
                message: serde_json::json!({
                    "kind": "task_output_expression_error",
                    "error": err_val,
                }),
            }),
        ),
    };

    if err.is_none() && invert_output {
        (invert_function_output(validated), None)
    } else {
        (validated, err)
    }
}

/// Client for executing Functions.
///
/// Orchestrates Function execution by flattening the Function and Profile
/// into executable tasks and running them (Vector Completions or nested
/// Functions) with streaming output support.
pub struct Client<
    CTXEXT,
    OPENROUTER,
    CLAUDEAGENTSDK,
    CODEXSDK,
    GEMINI,
    MOCK,
    ACUSG,
    VUSG,
    RETRG,
    RETRF,
    RETRM,
    FUSG,
> {
    /// Agent completions client for reasoning summaries.
    pub agent_client: Arc<crate::agent::completions::Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, GEMINI, MOCK, RETRG, RETRF, RETRM, ACUSG>>,
    /// Vector completions client for executing Vector Completion tasks.
    pub vector_client: Arc<
        vector::completions::Client<
            CTXEXT,
            OPENROUTER,
            CLAUDEAGENTSDK,
            CODEXSDK,
            GEMINI,
            MOCK,
            RETRG,
            RETRF,
            RETRM,
            ACUSG,
            VUSG,
        >,
    >,
    /// Router for fetching Function and Profile definitions.
    pub retrieve_router:
        Arc<crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>>,
    /// Handler for recording usage after execution.
    pub usage_handler: Arc<FUSG>,
}

impl<
    CTXEXT,
    OPENROUTER,
    CLAUDEAGENTSDK,
    CODEXSDK,
    GEMINI,
    MOCK,
    ACUSG,
    VUSG,
    RETRG,
    RETRF,
    RETRM,
    FUSG,
>
    Client<
        CTXEXT,
        OPENROUTER,
        CLAUDEAGENTSDK,
        CODEXSDK,
        GEMINI,
        MOCK,
        ACUSG,
        VUSG,
        RETRG,
        RETRF,
        RETRM,
        FUSG,
    >
{
    /// Creates a new Function execution client.
    pub fn new(
        agent_client: Arc<crate::agent::completions::Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, GEMINI, MOCK, RETRG, RETRF, RETRM, ACUSG>>,
        vector_client: Arc<
            vector::completions::Client<
                CTXEXT,
                OPENROUTER,
                CLAUDEAGENTSDK,
                CODEXSDK,
                GEMINI,
                MOCK,
                RETRG,
                RETRF,
                RETRM,
                ACUSG,
                VUSG,
            >,
        >,
        retrieve_router: Arc<
            crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>,
        >,
        usage_handler: Arc<FUSG>,
    ) -> Self {
        Self {
            agent_client,
            vector_client,
            retrieve_router,
            usage_handler,
        }
    }
}

impl<
    CTXEXT,
    OPENROUTER,
    CLAUDEAGENTSDK,
    CODEXSDK,
    GEMINI,
    MOCK,
    ACUSG,
    VUSG,
    RETRG,
    RETRF,
    RETRM,
    FUSG,
>
    Client<
        CTXEXT,
        OPENROUTER,
        CLAUDEAGENTSDK,
        CODEXSDK,
        GEMINI,
        MOCK,
        ACUSG,
        VUSG,
        RETRG,
        RETRF,
        RETRM,
        FUSG,
    >
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation> + Send + Sync + 'static,
    CLAUDEAGENTSDK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation> + Send + Sync + 'static,
    CODEXSDK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation> + Send + Sync + 'static,
    GEMINI: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::gemini::Agent, objectiveai_sdk::agent::gemini::Continuation> + Send + Sync + 'static,
    MOCK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation> + Send + Sync + 'static,
    ACUSG: crate::agent::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    VUSG: vector::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    FUSG: super::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
{
    /// Executes a Function and returns the complete response.
    ///
    /// Collects the full streaming response and records usage.
    pub async fn create_unary_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
    ) -> Result<
        objectiveai_sdk::functions::executions::response::unary::FunctionExecution,
        super::Error,
    > {
        let mut aggregate: Option<
            objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk,
        > = None;
        let mut stream =
            self.create_streaming_handle_usage(ctx, request).await?;
        while let Some(chunk) = stream.next().await {
            match &mut aggregate {
                Some(aggregate) => aggregate.push(&chunk),
                None => aggregate = Some(chunk),
            }
        }
        Ok(aggregate.unwrap().into())
    }

    /// Executes a Function with streaming output and records usage.
    ///
    /// Streams chunks as they become available and records usage after completion.
    ///
    /// Honours `request.invert`: when set, every chunk's outputs (root +
    /// nested function-execution tasks, recursively) are inverted before
    /// being forwarded to the consumer or aggregated for the usage handler.
    /// Inversion runs at this layer, *after* the inner client has finished
    /// evaluating expressions, so user-supplied expressions always see the
    /// original scores.
    pub async fn create_streaming_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
    ) -> Result<
        impl Stream<Item = objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk>
        + Send
        + Unpin
        + 'static,
        super::Error,
    >{
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let invert = request.invert.unwrap_or(false);
        tokio::spawn(async move {
            let mut aggregate: Option<
                objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk,
            > = None;
            let stream = match self
                .clone()
                .create_streaming(ctx.clone(), request.clone())
                .await
            {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            futures::pin_mut!(stream);
            while let Some(mut chunk) = stream.next().await {
                if invert {
                    invert_function_execution_chunk(&mut chunk);
                }
                match &mut aggregate {
                    Some(aggregate) => aggregate.push(&chunk),
                    None => aggregate = Some(chunk.clone()),
                }
                if tx.send(Ok(chunk)).is_err() {
                    ctx.cancel();
                }
            }
            drop(stream);
            drop(tx);
            let response: objectiveai_sdk::functions::executions::response::unary::FunctionExecution =
                aggregate.unwrap().into();
            if response.usage.any_usage() {
                self.usage_handler
                    .handle_usage(ctx, request, response)
                    .await;
            }
        });
        let mut stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        match stream.next().await {
            Some(Ok(chunk)) => {
                Ok(StreamOnce::new(chunk).chain(stream.map(Result::unwrap)))
            }
            Some(Err(e)) => Err(e),
            None => unreachable!(),
        }
    }
}

impl<
    CTXEXT,
    OPENROUTER,
    CLAUDEAGENTSDK,
    CODEXSDK,
    GEMINI,
    MOCK,
    ACUSG,
    VUSG,
    RETRG,
    RETRF,
    RETRM,
    FUSG,
>
    Client<
        CTXEXT,
        OPENROUTER,
        CLAUDEAGENTSDK,
        CODEXSDK,
        GEMINI,
        MOCK,
        ACUSG,
        VUSG,
        RETRG,
        RETRF,
        RETRM,
        FUSG,
    >
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation> + Send + Sync + 'static,
    CLAUDEAGENTSDK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation> + Send + Sync + 'static,
    CODEXSDK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation> + Send + Sync + 'static,
    GEMINI: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::gemini::Agent, objectiveai_sdk::agent::gemini::Continuation> + Send + Sync + 'static,
    MOCK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation> + Send + Sync + 'static,
    ACUSG: crate::agent::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    VUSG: vector::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    FUSG: Send + Sync + 'static,
{
    /// Executes a Function with streaming output.
    ///
    /// Fetches the Function and Profile, flattens them into tasks, and
    /// executes all tasks with streaming output. Handles reasoning summaries
    /// if requested.
    pub async fn create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
    ) -> Result<
        futures::stream::BoxStream<'static, objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk>,
        super::Error,
    >{
        // timestamp the completion
        let created = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // generate response id
        let response_id = response_id(created);

        let request_input = request.input.clone();

        // ── Split dispatch ─────────────────────────────────────────────
        //
        // Runs one execution per array element concurrently.
        //
        // Phase 1: concurrently create every inner stream. If *any* setup
        // fails, the whole call fails with that error.
        //
        // Phase 2: merge all inner streams via `select_all`, yielding each
        // chunk as it arrives — a slow input never blocks a fast one.
        if request.split.unwrap_or(false) {
            let elements = match request.input.clone() {
                objectiveai_sdk::functions::expression::InputValue::Array(arr) => arr,
                _ => return Err(super::Error::SplitInputNotArray),
            };

            // Phase 1: create all inner streams concurrently. First Err wins.
            // Each split element is its own sub-function-execution with a
            // freshly-minted `response_id`. The parent's `response_id` is
            // NOT passed down — it stays at the outer root level only.
            let setup_futs = elements.into_iter().enumerate().map(|(split_idx, element)| {
                let this = self.clone();
                let ctx = ctx.clone();
                let request = request.clone();
                let inner_response_id = self::response_id(created);
                async move {
                    this.execute_for_input(
                        ctx,
                        request,
                        element,
                        inner_response_id,
                        created,
                        Some(split_idx as u64),
                    )
                    .await
                    .map(move |stream| (split_idx, stream))
                }
            });
            let inner_streams = futures::future::try_join_all(setup_futs)
                .await?;
            let n = inner_streams.len();

            return Ok(async_stream::stream! {
                use futures::StreamExt as _;

                // Per-split outputs. Each slot defaults to an error; it gets
                // overwritten whenever that split_idx's inner stream yields a
                // chunk carrying an output (the last such wins). Root
                // `output` and `usage` are stripped on forwarded chunks
                // (mirrors Swiss strategy); `split_index` on the wrapped
                // task chunk preserves per-element attribution.
                let mut all_outputs: Vec<objectiveai_sdk::functions::expression::TaskOutputOwned> =
                    (0..n)
                        .map(|_| objectiveai_sdk::functions::expression::TaskOutputOwned::Err {
                            error: serde_json::Value::String("no output produced".to_string()),
                        })
                        .collect();
                let mut tasks_errors = false;
                let mut function_path = None;
                let mut profile_path = None;
                let mut object = objectiveai_sdk::functions::executions::response::streaming::Object::ScalarFunctionExecutionChunk;
                let mut total_usage = objectiveai_sdk::agent::completions::response::Usage::default();

                // Phase 2: merge every inner stream, tagging each chunk with
                // its split_idx. Chunks from any input are forwarded the
                // instant they arrive.
                type Tagged = std::pin::Pin<Box<dyn futures::Stream<
                    Item = (usize, objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk),
                > + Send>>;
                let tagged: Vec<Tagged> = inner_streams
                    .into_iter()
                    .map(|(split_idx, stream)| {
                        stream
                            .map(move |chunk| (split_idx, chunk))
                            .boxed() as Tagged
                    })
                    .collect();

                let mut merged = futures::stream::select_all(tagged);
                while let Some((split_idx, chunk)) = merged.next().await {
                    // capture function/profile paths and object from the first chunk we see
                    if function_path.is_none() {
                        function_path = chunk.function.clone();
                        profile_path = chunk.profile.clone();
                        object = chunk.object.clone();
                    }
                    if let Some(ref output) = chunk.output {
                        // last output wins per split_idx
                        all_outputs[split_idx] = output.output.clone();
                    }
                    if chunk.tasks_errors.unwrap_or(false) {
                        tasks_errors = true;
                    }
                    if let Some(chunk_usage) = &chunk.usage {
                        total_usage.push(chunk_usage);
                    }

                    // Wrap the inner chunk as a task chunk under the parent
                    // response_id. The inner chunk's own `id` (a unique
                    // fnexec-* per split element) travels inside `inner`.
                    //
                    // `index` is set to `split_idx`, not a monotonic chunk
                    // counter: `FunctionExecutionChunk::push_tasks` merges
                    // task chunks by `index`, so multiple chunks from the
                    // same split element must share an index to merge. A
                    // unique per-chunk index would make the aggregated
                    // `tasks` vector grow unbounded — O(N²) memory and I/O
                    // for any consumer that writes the aggregate on each
                    // chunk (e.g. the CLI log writer).
                    let object_for_chunk = chunk.object.clone();
                    let task_chunk = objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionTaskChunk {
                        index: split_idx as u64,
                        task_index: split_idx as u64,
                        task_path: vec![split_idx as u64],
                        swiss_pool_index: None,
                        swiss_round: None,
                        split_index: Some(split_idx as u64),
                        inner: chunk,
                    };

                    yield objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk {
                        id: response_id.clone(),
                        tasks: vec![
                            objectiveai_sdk::functions::executions::response::streaming::TaskChunk::FunctionExecution(task_chunk),
                        ],
                        tasks_errors: if tasks_errors { Some(true) } else { None },
                        reasoning: None,
                        output: None,
                        error: None,
                        created,
                        function: function_path.clone(),
                        profile: profile_path.clone(),
                        object: object_for_chunk,
                        usage: None,
                    };
                }

                // combine outputs — find the first non-error to determine the variant
                let first_ok = all_outputs
                    .iter()
                    .find(|o| !matches!(o, objectiveai_sdk::functions::expression::TaskOutputOwned::Err { .. }));
                let combined = match first_ok {
                    None => {
                        objectiveai_sdk::functions::expression::TaskOutputOwned::Err {
                            error: serde_json::Value::String("no split outputs".to_string()),
                        }
                    }
                    Some(objectiveai_sdk::functions::expression::TaskOutputOwned::Scalar(_)) => {
                        objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(
                            all_outputs.into_iter().map(|o| match o {
                                objectiveai_sdk::functions::expression::TaskOutputOwned::Scalar(d) => d,
                                _ => rust_decimal::Decimal::ZERO,
                            }).collect()
                        )
                    }
                    Some(objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(_)) => {
                        objectiveai_sdk::functions::expression::TaskOutputOwned::Vectors(
                            all_outputs.into_iter().map(|o| match o {
                                objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(v) => v,
                                _ => Vec::new(),
                            }).collect()
                        )
                    }
                    _ => {
                        objectiveai_sdk::functions::expression::TaskOutputOwned::Err {
                            error: serde_json::Value::String("unexpected output type in split".to_string()),
                        }
                    }
                };

                yield objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk {
                    id: response_id.clone(),
                    tasks: Vec::new(),
                    tasks_errors: if tasks_errors { Some(true) } else { None },
                    reasoning: None,
                    output: Some(objectiveai_sdk::functions::executions::response::Output { output: combined }),
                    error: None,
                    created,
                    function: function_path,
                    profile: profile_path,
                    object,
                    usage: Some(total_usage),
                };
            }.boxed());
        }

        // ── Single execution (no split) ───────────────────────────────
        self.execute_for_input(
            ctx,
            request,
            request_input,
            response_id,
            created,
            None,
        ).await.map(|s| s.boxed())
    }

    /// Executes a single function for one input. Contains strategy dispatch
    /// (Swiss System vs default) and reasoning summary handling.
    async fn execute_for_input(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
        input: objectiveai_sdk::functions::expression::InputValue,
        response_id: String,
        created: u64,
        split_index: Option<u64>,
    ) -> Result<
        impl Stream<Item = objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk>
        + Send
        + 'static,
        super::Error,
    > {
        // validate that input_split and input_merge are present if strategy is Swiss
        let inline_function = match &request.function {
            objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional::Inline(f) => Some(f.clone().transpile()),
            _ => None,
        };
        match (&request.strategy, &inline_function) {
            (
                Some(
                    objectiveai_sdk::functions::executions::request::Strategy::SwissSystem {
                        ..
                    },
                ),
                Some(objectiveai_sdk::functions::InlineFunction::Vector {
                    input_split: Some(_),
                    input_merge: Some(_),
                    ..
                })
            )=> { }
            (
                Some(
                    objectiveai_sdk::functions::executions::request::Strategy::SwissSystem {
                        ..
                    },
                ),
                Some(_)
            ) => {
                return Err(super::Error::InvalidFunctionForStrategy(
                    "With 'swiss_system' strategy, Inline Function must be vector with both `input_split` and `input_merge` present."
                        .to_string(),
                ));
            }
            _ => { }
        }

        // fetch function flat task profile
        let mut ftp = functions::get_flat_task_profile(
                &ctx,
                Vec::new(),
                request.function.clone(),
                request.profile.clone(),
                input.clone(),
                None,
                false,
                self.retrieve_router.clone(),
                std::collections::HashSet::new(),
            )
            .await?;

        // validate that ftp type is Vector if strategy is Swiss
        match (&request.strategy, &ftp.r#type) {
            (
                Some(
                    objectiveai_sdk::functions::executions::request::Strategy::SwissSystem {
                        ..
                    },
                ),
                functions::FunctionType::Scalar,
            ) => {
                return Err(super::Error::InvalidFunctionForStrategy(
                    "With 'swiss_system' strategy, Function must be of type 'vector'."
                        .to_string(),
                ));
            }
            _ => { }
        }

        // take description from ftp
        let description = ftp.description.take();

        // reasonong data
        let reasoning = request.reasoning.is_some();
        let mut reasoning_data = if reasoning {
            Some((
                HashMap::<
                    String,
                    objectiveai_sdk::functions::executions::response::streaming::VectorCompletionTaskChunk,
                >::new(),
                {
                    let mut confidence_responses: Vec<ConfidenceResponse> =
                        Vec::new();
                    let mut index_map: HashMap<Vec<u64>, Vec<usize>> =
                        HashMap::new();
                    for vector_completion_ftp in ftp
                        .tasks
                        .iter()
                        .filter_map(|task| task.as_ref())
                        .flat_map(|task| task.vector_completion_ftps())
                    {
                        let mut completion_index_map = Vec::with_capacity(
                            vector_completion_ftp.responses.len(),
                        );
                        for response in &vector_completion_ftp.responses {
                            let mut response = response.clone();
                            response.prepare();
                            let response_string =
                                serde_json::to_string(&response)
                                    .unwrap_or_default();
                            if response_string.is_empty() {
                                continue;
                            }
                            let mut hasher = ahash::AHasher::default();
                            hasher.write(response_string.as_bytes());
                            let response_hash = hasher.finish();
                            let mut found = false;
                            for (i, confidence_response) in
                                confidence_responses.iter_mut().enumerate()
                            {
                                if confidence_response.response_hash
                                    == response_hash
                                {
                                    confidence_response.paths.push(
                                        vector_completion_ftp.path.clone(),
                                    );
                                    confidence_response.confidence_count +=
                                        rust_decimal::Decimal::ONE;
                                    completion_index_map.push(i);
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                completion_index_map
                                    .push(confidence_responses.len());
                                confidence_responses.push(ConfidenceResponse {
                                    response_hash,
                                    paths: vec![
                                        vector_completion_ftp.path.clone(),
                                    ],
                                    confidence_count:
                                        rust_decimal::Decimal::ONE,
                                    response,
                                    confidence: rust_decimal::Decimal::ZERO,
                                    reasoning: Vec::new(),
                                });
                            }
                        }
                        index_map.insert(
                            vector_completion_ftp.path.clone(),
                            completion_index_map,
                        );
                    }
                    (index_map, confidence_responses)
                },
                None::<
                    objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk,
                >,
            ))
        } else {
            None
        };

        // ── Swiss System Strategy ──────────────────────────────────────
        //
        // A tournament-style ranking algorithm for vector functions that avoids
        // the O(N²) cost of comparing all items simultaneously. Instead of one
        // large vector completion over all N items, items are split into small
        // pools and scored across multiple rounds.
        //
        // Overview:
        //   1. `input_split` breaks the original input into N individual sub-inputs.
        //   2. Sub-inputs are grouped into pools of `pool` size. If len % pool == 1,
        //      pools are sized pool+1 instead to avoid single-item trailing chunks
        //      (a single item can't be meaningfully scored against itself).
        //   3. `input_merge` reconstitutes each pool's sub-inputs into a single
        //      well-formed input for the function.
        //   4. A flat task profile is compiled per pool (same function & profile,
        //      different input). Function/profile fetches hit the per-request
        //      dedup cache after the first call — only expression compilation
        //      (which is input-dependent) is repeated.
        //   5. All pools within a round execute concurrently via `select_all`.
        //   6. After each round, scores are mapped back to original indices,
        //      cumulative scores are updated, and items are re-sorted so that
        //      similarly-ranked items compete in the next round — this is the
        //      core Swiss System property.
        //   7. After all rounds, the final output is the normalized average of
        //      per-round scores, in original input order.
        //
        // Retry tokens are only captured from the first round. Errors in
        // subsequent rounds are non-fatal: they're stored and included in the
        // final output chunk rather than aborting the entire execution.
        //
        // Index tracking:
        //   - `current_to_original`: maps current sorted position → original index.
        //     Updated after each round's re-sort.
        //   - `pool_chunk_sizes`: sizes of each pool in the current round, used to
        //     map pool-local indices back to positions in the sorted order.
        //   - `cumulative_scores`: running total per original index, used for
        //     re-sorting between rounds.
        let choice_indexer = Arc::new(ChoiceIndexer::new(0));
        if let Some(
            objectiveai_sdk::functions::executions::request::Strategy::SwissSystem {
                pool,
                rounds,
            }
        ) = &request.strategy {
            // take and unwrap input_split and input_merge
            let (input_split, input_merge) = match &ftp.r#type {
                functions::FunctionType::Vector {
                    input_split,
                    input_merge,
                    ..
                } => (
                    input_split.clone().expect("missing input_split"),
                    input_merge.clone().expect("missing input_merge"),
                ),
                _ => unreachable!(),
            };

            // validate pool and rounds
            let pool = pool.unwrap_or(10);
            let rounds = rounds.unwrap_or(3);
            if pool <= 1 || rounds == 0 {
                return Err(super::Error::InvalidStrategy(
                    "For 'swiss_system' strategy, 'pool' must be > 1 and 'rounds' must be > 0."
                        .to_string(),
                ));
            }

            // Split the original input into N individual sub-inputs (one per item to rank).
            // e.g., for 20 items with pool=5, this produces 20 sub-inputs.
            let split_input: Vec<objectiveai_sdk::functions::expression::InputValue> = input_split.compile_one(
                &objectiveai_sdk::functions::expression::Params::Ref(
                    objectiveai_sdk::functions::expression::ParamsRef {
                        input: &input,
                        output: None,
                        map: None,
                    }
                ),
            ).map_err(super::Error::from)?;

            // ── Round 1: build flat task profiles per pool ──────────────
            // Group sub-inputs into pool-sized chunks, merge each chunk back
            // into a single input via `input_merge`, and compile a flat task
            // profile for each pool. All pools are compiled concurrently.
            let mut ftp_futs = Vec::with_capacity(split_input.len() / pool + 1);
            let mut pool_chunk_sizes: Vec<usize> = Vec::with_capacity(split_input.len() / pool + 1);
            let chunks = split_input.chunks(
                if split_input.len() % pool == 1 {
                    pool + 1
                } else {
                    pool
                }
            );
            for chunk in chunks {
                pool_chunk_sizes.push(chunk.len());
                let joined_input: objectiveai_sdk::functions::expression::InputValue = input_merge.compile_one(
                    &objectiveai_sdk::functions::expression::Params::Owned(
                        objectiveai_sdk::functions::expression::ParamsOwned {
                            input: objectiveai_sdk::functions::expression::InputValue::Array(
                                chunk.to_vec(),
                            ),
                            output: None,
                            map: None,
                        }
                    )
                ).map_err(super::Error::from)?;
                ftp_futs.push(functions::get_flat_task_profile(
                    &ctx,
                    Vec::new(),
                    request.function.clone(),
                    request.profile.clone(),
                    joined_input,
                    None,
                    false,
                    self.retrieve_router.clone(),
                    std::collections::HashSet::new(),
                ));
            }
            let mut ftps = futures::future::try_join_all(ftp_futs).await?;

            // setup reasoning data for Swiss system
            let (mut swiss_vector_completions, mut swiss_index_maps, swiss_confidence_responses) = if reasoning {
                // extract confidence_responses from reasoning_data (built from original ftp)
                let (_, (_, confidence_responses), _) = reasoning_data.take().unwrap();

                // build index_maps for initial FTPs (round 1)
                let mut index_maps: HashMap<(u64, usize), HashMap<Vec<u64>, Vec<usize>>> = HashMap::new();
                for (pool_idx, ftp) in ftps.iter().enumerate() {
                    let mut ftp_index_map: HashMap<Vec<u64>, Vec<usize>> = HashMap::new();
                    for vector_completion_ftp in ftp
                        .tasks
                        .iter()
                        .filter_map(|task| task.as_ref())
                        .flat_map(|task| task.vector_completion_ftps())
                    {
                        let mut completion_index_map = Vec::with_capacity(
                            vector_completion_ftp.responses.len(),
                        );
                        for response in &vector_completion_ftp.responses {
                            let mut response = response.clone();
                            response.prepare();
                            let response_string =
                                serde_json::to_string(&response).unwrap_or_default();
                            if response_string.is_empty() {
                                continue;
                            }
                            let mut hasher = ahash::AHasher::default();
                            hasher.write(response_string.as_bytes());
                            let response_hash = hasher.finish();
                            // find matching confidence_response by hash
                            for (i, confidence_response) in confidence_responses.iter().enumerate() {
                                if confidence_response.response_hash == response_hash {
                                    completion_index_map.push(i);
                                    break;
                                }
                            }
                        }
                        ftp_index_map.insert(
                            vector_completion_ftp.path.clone(),
                            completion_index_map,
                        );
                    }
                    index_maps.insert((0, pool_idx), ftp_index_map);
                }

                (
                    Some(HashMap::<String, (u64, usize, objectiveai_sdk::functions::executions::response::streaming::VectorCompletionTaskChunk)>::new()),
                    Some(index_maps),
                    Some(confidence_responses),
                )
            } else {
                (None, None, None)
            };

            // identify the response type
            let object = match ftp.r#type {
                functions::FunctionType::Vector { .. } =>
                    objectiveai_sdk::functions::executions::response::streaming::Object::VectorFunctionExecutionChunk,
                _ => unreachable!(),
            };

            // track usage
            let mut usage =
                objectiveai_sdk::agent::completions::response::Usage::default();

            // track original indices: current_position -> original_index
            let num_items = split_input.len();
            let mut current_to_original: Vec<usize> = (0..num_items).collect();

            // track cumulative scores per original index (for sorting)
            let mut cumulative_scores: Vec<rust_decimal::Decimal> =
                vec![rust_decimal::Decimal::ZERO; num_items];

            // track outputs per round: round -> (original_index -> score)
            let mut round_outputs: Vec<Vec<rust_decimal::Decimal>> = Vec::with_capacity(rounds as usize);

            // identifiers
            let function = ftp.function_path;
            let profile = ftp.profile_path;

            // track whether child errors occurred
            let mut tasks_errors = false;

            Ok(futures::future::Either::Left(async_stream::stream! {
                // track errors from subsequent rounds to include in final output
                let mut subsequent_round_error: Option<objectiveai_sdk::error::ResponseError> = None;

                // monotonic task index across all pools and rounds
                let mut swiss_task_index: u64 = 0;

                // ── Main round loop ────────────────────────────────────
                // Each iteration: execute all pools, collect scores, re-sort
                // items by cumulative score, re-pool for the next round.
                'rounds: for current_round in 0..rounds {
                    let is_last_round = current_round == rounds - 1;

                    // Execute all pools for this round concurrently. Each pool
                    // produces a stream of chunks (vector completion results,
                    // function execution chunks, retry tokens).
                    let mut streams = Vec::with_capacity(ftps.len());

                    for (i, ftp) in ftps.drain(..).enumerate() {
                        let pool_task_index = swiss_task_index;
                        swiss_task_index += 1;

                        streams.push((
                            i,
                            self.clone().execute_function_ftp_streaming(
                                ctx.clone(),
                                request.clone(),
                                ftp,
                                None,
                                created,
                                pool_task_index,
                                choice_indexer.clone(),
                                Some(current_round as u64),
                                Some(i as u64),
                                split_index,
                            ).boxed(),
                        ));
                    }

                    // collect outputs from this round, keyed by pool index
                    let mut pool_outputs: HashMap<usize, Vec<rust_decimal::Decimal>> = HashMap::new();

                    // stream and collect results
                    let stream = futures::stream::select_all(
                        streams.into_iter().map(|(pool_idx, stream)| {
                            stream.map(move |chunk| (pool_idx, chunk))
                        })
                    );
                    futures::pin_mut!(stream);

                    while let Some((pool_idx, chunk)) = stream.next().await {
                        match chunk {
                            FtpStreamChunk::FunctionExecutionChunk(chunk) => {
                                // check for output
                                if let Some(ref output) = chunk.inner.output {
                                    if let objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(scores) = &output.output {
                                        pool_outputs.insert(pool_idx, scores.clone());
                                    }
                                }

                                // track usage and errors
                                tasks_errors |= chunk.inner.error.is_some()
                                    || chunk.inner.tasks_errors.unwrap_or(false);
                                if let Some(chunk_usage) = &chunk.inner.usage {
                                    usage.push(chunk_usage);
                                }

                                // yield chunk
                                yield objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk {
                                    id: response_id.clone(),
                                    tasks: vec![
                                        objectiveai_sdk::functions::executions::response::streaming::TaskChunk::FunctionExecution(
                                            chunk,
                                        ),
                                    ],
                                    tasks_errors: if tasks_errors {
                                        Some(true)
                                    } else {
                                        None
                                    },
                                    reasoning: None,
                                    output: None,
                                    error: None,
                                    created,
                                    function: function.clone(),
                                    profile: profile.clone(),
                                    object,
                                    usage: None,
                                };
                            }
                            FtpStreamChunk::OutputChunk { .. } => {}
                            FtpStreamChunk::VectorCompletionTaskChunk(chunk) => {
                                // track usage and errors
                                tasks_errors |= chunk.error.is_some();
                                if let Some(chunk_usage) = &chunk.inner.usage {
                                    usage.push(chunk_usage);
                                }
                                // aggregate for reasoning
                                if let Some(vector_completions) = &mut swiss_vector_completions {
                                    if !chunk.inner.id.is_empty() {
                                        match vector_completions.get_mut(&chunk.inner.id) {
                                            Some((_, _, existing_chunk)) => {
                                                existing_chunk.push(&chunk);
                                            }
                                            None => {
                                                vector_completions.insert(
                                                    chunk.inner.id.clone(),
                                                    (current_round as u64, pool_idx, chunk.clone()),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Score remapping ────────────────────────────────────
                    // Pool outputs are in sorted order (after round 1). Map each
                    // score back to its original index using `current_to_original`,
                    // then accumulate into `cumulative_scores` for re-sorting.
                    let mut this_round_scores: Vec<rust_decimal::Decimal> =
                        vec![rust_decimal::Decimal::ZERO; num_items];

                    let mut position = 0usize;
                    for (pool_idx, &chunk_size) in pool_chunk_sizes.iter().enumerate() {
                        if let Some(scores) = pool_outputs.get(&pool_idx) {
                            for (local_idx, &score) in scores.iter().enumerate() {
                                let current_pos = position + local_idx;
                                if current_pos < current_to_original.len() {
                                    let original_idx = current_to_original[current_pos];
                                    this_round_scores[original_idx] = score;
                                    cumulative_scores[original_idx] += score;
                                }
                            }
                        }
                        // always advance by expected chunk size, even if pool had no output
                        position += chunk_size;
                    }
                    round_outputs.push(this_round_scores);

                    // ── Re-sort and re-pool for next round ─────────────────
                    // Sort items by cumulative score (descending) so similarly-
                    // ranked items land in the same pool. This is the Swiss System
                    // property: strong items compete with strong, weak with weak,
                    // producing more informative comparisons each round.
                    if !is_last_round {
                        let mut sorted_indices: Vec<usize> = (0..num_items).collect();
                        sorted_indices.sort_by(|&a, &b| {
                            cumulative_scores[b].cmp(&cumulative_scores[a])
                                .then_with(|| a.cmp(&b))
                        });

                        // update current_to_original mapping
                        // sorted_indices[new_pos] = original_idx
                        current_to_original = sorted_indices.clone();

                        // rebuild split_input in new sorted order
                        let sorted_split_input: Vec<objectiveai_sdk::functions::expression::InputValue> =
                            sorted_indices.iter()
                                .map(|&orig_idx| split_input[orig_idx].clone())
                                .collect();

                        // re-chunk and fetch new FTPs
                        let chunks = sorted_split_input.chunks(
                            if sorted_split_input.len() % pool == 1 {
                                pool + 1
                            } else {
                                pool
                            }
                        );

                        // update pool_chunk_sizes for this round
                        pool_chunk_sizes.clear();
                        let mut ftp_futs = Vec::with_capacity(chunks.len());
                        for chunk in chunks {
                            pool_chunk_sizes.push(chunk.len());
                            let joined_input: objectiveai_sdk::functions::expression::InputValue = match input_merge.compile_one(
                                &objectiveai_sdk::functions::expression::Params::Owned(
                                    objectiveai_sdk::functions::expression::ParamsOwned {
                                        input: objectiveai_sdk::functions::expression::InputValue::Array(
                                            chunk.to_vec(),
                                        ),
                                        output: None,
                                        map: None,
                                    }
                                )
                            ) {
                                Ok(input) => input,
                                Err(e) => {
                                    // store error for final output and break
                                    subsequent_round_error = Some(objectiveai_sdk::error::ResponseError::from(
                                        &super::Error::from(e)
                                    ));
                                    tasks_errors = true;
                                    break 'rounds;
                                }
                            };
                            ftp_futs.push(functions::get_flat_task_profile(
                                &ctx,
                                Vec::new(),
                                request.function.clone(),
                                request.profile.clone(),
                                joined_input,
                                None,
                                false,
                                self.retrieve_router.clone(),
                                std::collections::HashSet::new(),
                            ));
                        }

                        ftps = match futures::future::try_join_all(ftp_futs).await {
                            Ok(new_ftps) => new_ftps,
                            Err(e) => {
                                // store error for final output and break
                                subsequent_round_error = Some(objectiveai_sdk::error::ResponseError::from(&e));
                                tasks_errors = true;
                                break 'rounds;
                            }
                        };

                        // build index_maps for new FTPs (next round)
                        if let (Some(index_maps), Some(confidence_responses)) = (&mut swiss_index_maps, &swiss_confidence_responses) {
                            let next_round = current_round + 1;
                            for (pool_idx, ftp) in ftps.iter().enumerate() {
                                let mut ftp_index_map: HashMap<Vec<u64>, Vec<usize>> = HashMap::new();
                                for vector_completion_ftp in ftp
                                    .tasks
                                    .iter()
                                    .filter_map(|task| task.as_ref())
                                    .flat_map(|task| task.vector_completion_ftps())
                                {
                                    let mut completion_index_map = Vec::with_capacity(
                                        vector_completion_ftp.responses.len(),
                                    );
                                    for response in &vector_completion_ftp.responses {
                                        let mut response = response.clone();
                                        response.prepare();
                                        let response_string =
                                            serde_json::to_string(&response).unwrap_or_default();
                                        if response_string.is_empty() {
                                            continue;
                                        }
                                        let mut hasher = ahash::AHasher::default();
                                        hasher.write(response_string.as_bytes());
                                        let response_hash = hasher.finish();
                                        // find matching confidence_response by hash
                                        for (i, confidence_response) in confidence_responses.iter().enumerate() {
                                            if confidence_response.response_hash == response_hash {
                                                completion_index_map.push(i);
                                                break;
                                            }
                                        }
                                    }
                                    ftp_index_map.insert(
                                        vector_completion_ftp.path.clone(),
                                        completion_index_map,
                                    );
                                }
                                index_maps.insert((next_round as u64, pool_idx), ftp_index_map);
                            }
                        }
                    }
                }

                // ── Final output ──────────────────────────────────────────
                // Average each item's scores across all rounds, then normalize
                // to sum to 1. Scores are already indexed by original position.
                let num_rounds = round_outputs.len();
                let mut final_output: Vec<rust_decimal::Decimal> = vec![rust_decimal::Decimal::ZERO; num_items];

                if num_rounds > 0 {
                    let num_rounds_dec = rust_decimal::Decimal::from(num_rounds as u64);
                    for original_idx in 0..num_items {
                        let mut sum = rust_decimal::Decimal::ZERO;
                        for round in &round_outputs {
                            sum += round[original_idx];
                        }
                        final_output[original_idx] = sum / num_rounds_dec;
                    }

                    // normalize to sum to 1
                    let total: rust_decimal::Decimal = final_output.iter().copied().sum();
                    if total > rust_decimal::Decimal::ZERO {
                        for score in &mut final_output {
                            *score /= total;
                        }
                    }
                }

                // ── Reasoning summary ─────────────────────────────────────
                // If reasoning was requested, aggregate confidence scores and
                // reasoning text from all vector completion chunks across all
                // rounds, then generate a summary via a chat completion.
                if let (Some(vector_completions), Some(index_maps), Some(mut confidence_responses)) =
                    (swiss_vector_completions, swiss_index_maps, swiss_confidence_responses)
                {
                    // unpack reasoning params
                    let objectiveai_sdk::functions::executions::request::Reasoning {
                        agent,
                    } = request.reasoning.as_ref().unwrap();

                    // iterate over vector completion chunks
                    for (_, (round, pool_idx, mut vector_completion)) in vector_completions.into_iter() {
                        // get index_map for this round/pool
                        if let Some(ftp_index_map) = index_maps.get(&(round, pool_idx)) {
                            if let Some(indices) = ftp_index_map.get(&vector_completion.task_path) {
                                for (i, score) in vector_completion
                                    .inner
                                    .scores
                                    .iter()
                                    .enumerate()
                                {
                                    if let Some(&idx) = indices.get(i) {
                                        confidence_responses[idx].confidence += *score;
                                    }
                                }
                                for vote in vector_completion.inner.votes {
                                    if let Some(completion_index) = vote.completion_index {
                                        let mut winning_index: usize = 0;
                                        let mut highest_vote = rust_decimal::Decimal::ZERO;
                                        for (i, &score) in vote.vote.iter().enumerate() {
                                            if score > highest_vote {
                                                highest_vote = score;
                                                winning_index = i;
                                            }
                                        }
                                        if let Some(&idx) = indices.get(winning_index) {
                                            let confidence_response = &mut confidence_responses[idx];
                                            let completion = vector_completion
                                                .inner
                                                .completions
                                                .iter_mut()
                                                .find(|c| c.index == completion_index)
                                                .expect("missing completion for vote completion index");
                                            // Extract reasoning from the first assistant message chunk
                                            if let Some(objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(assistant)) = completion.inner.messages.first_mut() {
                                                if let Some(reasoning) = assistant.reasoning.take() {
                                                    confidence_response.reasoning.push(reasoning);
                                                }
                                                if let Some(objectiveai_sdk::agent::completions::message::RichContent::Text(content)) = assistant.content.take()
                                                    && let Ok(crate::vector::completions::ResponseKey {
                                                        _think: Some(reasoning),
                                                        ..
                                                    }) = serde_json::from_str(&content)
                                                {
                                                    confidence_response.reasoning.push(reasoning);
                                                }
                                                if let Some(tool_calls) = assistant.tool_calls.take() {
                                                    for tool_call in tool_calls {
                                                        if let Some(objectiveai_sdk::agent::completions::message::AssistantToolCallFunctionDelta {
                                                            arguments: Some(arguments),
                                                            ..
                                                        }) = tool_call.function
                                                            && let Ok(crate::vector::completions::ResponseKey {
                                                                _think: Some(reasoning),
                                                                ..
                                                            }) = serde_json::from_str(&arguments)
                                                        {
                                                            confidence_response.reasoning.push(reasoning);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // normalize response confidences
                    for confidence_response in &mut confidence_responses {
                        if confidence_response.confidence_count > rust_decimal::Decimal::ONE {
                            confidence_response.confidence /= confidence_response.confidence_count;
                        }
                    }

                    // create a chat completion summarizing the reasoning
                    let reasoning_stream = self.create_reasoning_summary_streaming(
                        ctx,
                        request.clone(),
                        agent.clone(),
                        description,
                        objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(final_output.clone()),
                        confidence_responses,
                    ).await;

                    // yield reasoning chunks
                    futures::pin_mut!(reasoning_stream);
                    while let Some(chunk) = reasoning_stream.next().await {
                        // collect usage
                        if let Some(chunk_usage) = &chunk.inner.usage {
                            usage.push(chunk_usage);
                        }

                        // yield chunk
                        yield objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk {
                            id: response_id.clone(),
                            tasks: Vec::new(),
                            tasks_errors: if tasks_errors {
                                Some(true)
                            } else {
                                None
                            },
                            reasoning: Some(chunk),
                            output: None,
                            error: None,
                            created,
                            function: function.clone(),
                            profile: profile.clone(),
                            object,
                            usage: None,
                        };
                    }
                }

                // yield final output chunk
                yield objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk {
                    id: response_id.clone(),
                    tasks: Vec::new(),
                    tasks_errors: if tasks_errors {
                        Some(true)
                    } else {
                        None
                    },
                    reasoning: None,
                    output: Some(objectiveai_sdk::functions::executions::response::Output { output: objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(final_output) }),
                    error: subsequent_round_error,
                    created,
                    function,
                    profile,
                    object,
                    usage: Some(usage),
                };
            }))
        } else {
            // get function stream
            let stream = self
                .clone()
                .execute_function_ftp_streaming(
                    ctx.clone(),
                    request.clone(),
                    ftp,
                    Some(response_id.clone()),
                    created,
                    0,
                    choice_indexer,
                    None,
                    None,
                    split_index,
                );

            Ok(futures::future::Either::Right(async_stream::stream! {
                futures::pin_mut!(stream);
                // stream all chunks
                while let Some(
                    FtpStreamChunk::FunctionExecutionChunk(chunk)
                ) = stream.next().await {
                    // handle reasoning tasks if needed
                    if reasoning {
                        // unwrap reasoning data
                        let (
                            vector_completions,
                            _,
                            final_chunk,
                        ) = &mut reasoning_data
                            .as_mut()
                            .unwrap();
                        // aggregate vector completions
                        for chunk in chunk.inner.vector_completion_tasks() {
                            if !chunk.inner.id.is_empty() {
                                match vector_completions.get_mut(&chunk.inner.id) {
                                    Some(existing_chunk) => {
                                        existing_chunk.push(chunk);
                                    }
                                    None => {
                                        let _ = vector_completions.insert(
                                            chunk.inner.id.clone(),
                                            chunk.clone(),
                                        );
                                    }
                                }
                            }
                        }
                        // stash the final chunk
                        if chunk.inner.output.is_some() {
                            // will be returned after reasoning summary
                            *final_chunk = Some(chunk.inner);
                        } else {
                            // yield chunk
                            yield chunk.inner;
                        }
                    } else {
                        // yield chunk
                        yield chunk.inner;
                    }
                }

                // handle reasoning
                if reasoning {
                    // unpack reasoning data
                    let objectiveai_sdk::functions::executions::request::Reasoning {
                        agent,
                    } = request.reasoning.as_ref().unwrap();
                    let (
                        vector_completions,
                        (
                            index_map,
                            mut confidence_responses,
                        ),
                        final_chunk,
                    ) = reasoning_data.unwrap();
                    let mut final_chunk = final_chunk.unwrap();

                    // iterate over vector completion chat completions
                    for mut vector_completion in vector_completions.into_values() {
                        let indices = index_map.get(&vector_completion.task_path)
                            .expect("missing index map for vector completion task path");
                        for (i, score) in vector_completion
                            .inner
                            .scores
                            .iter()
                            .enumerate()
                        {
                            let confidence_response =
                                &mut confidence_responses[indices[i]];
                            confidence_response.confidence += *score;
                        }
                        for vote in vector_completion.inner.votes {
                            if let Some(completion_index) = vote.completion_index {
                                let mut winning_index: usize = 0;
                                let mut highest_vote =
                                    rust_decimal::Decimal::ZERO;
                                for (i, &score) in vote.vote.iter().enumerate() {
                                    if score > highest_vote {
                                        highest_vote = score;
                                        winning_index = i;
                                    }
                                }
                                let confidence_response =
                                    &mut confidence_responses[indices[winning_index]];
                                let completion = vector_completion
                                    .inner
                                    .completions
                                    .iter_mut()
                                    .find(|c| c.index == completion_index)
                                    .expect(
                                        "missing completion for vote completion index",
                                    );
                                // Extract reasoning from the first assistant message chunk
                                if let Some(objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(assistant)) = completion.inner.messages.first_mut() {
                                    if let Some(reasoning) = assistant.reasoning.take() {
                                        confidence_response.reasoning.push(reasoning);
                                    }
                                    if let Some(objectiveai_sdk::agent::completions::message::RichContent::Text(content)) = assistant.content.take()
                                        && let Ok(crate::vector::completions::ResponseKey {
                                            _think: Some(reasoning),
                                            ..
                                        }) = serde_json::from_str(&content)
                                    {
                                        confidence_response.reasoning.push(reasoning);
                                    }
                                    if let Some(tool_calls) = assistant.tool_calls.take() {
                                        for tool_call in tool_calls {
                                            if let Some(objectiveai_sdk::agent::completions::message::AssistantToolCallFunctionDelta {
                                                arguments: Some(arguments),
                                                ..
                                            }) = tool_call.function
                                                && let Ok(crate::vector::completions::ResponseKey {
                                                    _think: Some(reasoning),
                                                    ..
                                                }) = serde_json::from_str(&arguments)
                                            {
                                                confidence_response.reasoning.push(
                                                    reasoning,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // normalize response confidences
                    for confidence_response in &mut confidence_responses {
                        if confidence_response.confidence_count
                            > rust_decimal::Decimal::ONE
                        {
                            confidence_response.confidence /= confidence_response
                                .confidence_count;
                        }
                    }

                    // create a chat completion summarizing the reasoning
                    let stream = self.create_reasoning_summary_streaming(
                        ctx,
                        request.clone(),
                        agent.clone(),
                        description,
                        final_chunk.output.clone().expect("missing output").output,
                        confidence_responses,
                    ).await;

                    // yield chunks
                    futures::pin_mut!(stream);
                    while let Some(chunk) = stream.next().await {
                        // collect usage
                        if let Some(chunk_usage) = &chunk.inner.usage {
                            if let Some(usage) = &mut final_chunk.usage {
                                usage.push(chunk_usage);
                            } else {
                                let mut usage = objectiveai_sdk::agent::completions::response::Usage::default();
                                usage.push(chunk_usage);
                                final_chunk.usage = Some(usage);
                            }
                        }

                        // yield chunk
                        yield objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk {
                            id: final_chunk.id.clone(),
                            tasks: Vec::new(),
                            tasks_errors: final_chunk.tasks_errors,
                            reasoning: Some(chunk),
                            output: None,
                            error: None,
                            created: final_chunk.created,
                            function: final_chunk.function.clone(),
                            profile: final_chunk.profile.clone(),
                            object: final_chunk.object.clone(),
                            usage: None,
                        };
                    }

                    // yield final chunk
                    yield final_chunk;
                }
            }))
        }
    }

    fn execute_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
        ftp: functions::FlatTaskProfile,
        created: u64,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
        swiss_round: Option<u64>,
        swiss_pool_index: Option<u64>,
        split_index: Option<u64>,
    ) -> futures::stream::BoxStream<'static, FtpStreamChunk> {
        match ftp {
            functions::FlatTaskProfile::Function(function_ftp) => self
                .clone()
                .execute_function_ftp_streaming(
                    ctx,
                    request,
                    function_ftp,
                    None,
                    created,
                    task_index,
                    choice_indexer,
                    swiss_round,
                    swiss_pool_index,
                    split_index,
                )
                .boxed(),
            functions::FlatTaskProfile::MapFunction(map_function_ftp) => self
                .clone()
                .execute_map_function_ftp_streaming(
                    ctx,
                    request,
                    map_function_ftp,
                    created,
                    task_index,
                    choice_indexer,
                    swiss_round,
                    swiss_pool_index,
                    split_index,
                )
                .boxed(),
            functions::FlatTaskProfile::VectorCompletion(vector_ftp) => {
                futures::stream::once(
                    self.clone().execute_vector_ftp_streaming(
                        ctx,
                        request,
                        vector_ftp,
                        task_index,
                        choice_indexer,
                    ),
                )
                .flatten()
                .boxed()
            }
            functions::FlatTaskProfile::MapVectorCompletion(map_vector_ftp) => {
                futures::stream::once(
                    self.clone().execute_map_vector_ftp_streaming(
                        ctx,
                        request,
                        map_vector_ftp,
                        task_index,
                        choice_indexer,
                    ),
                )
                .flatten()
                .boxed()
            }
            functions::FlatTaskProfile::PlaceholderScalarFunction(_ftp) => {
                let output = objectiveai_sdk::functions::expression::TaskOutputOwned::Scalar(
                    rust_decimal::Decimal::new(5, 1), // 0.5
                );
                futures::stream::once(async move {
                    FtpStreamChunk::OutputChunk {
                        task_index,
                        output,
                    }
                })
                .boxed()
            }
            functions::FlatTaskProfile::MapPlaceholderScalarFunction(ftp) => {
                let output = objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(
                    ftp.placeholders
                        .iter()
                        .map(|_| rust_decimal::Decimal::new(5, 1))
                        .collect(),
                );
                futures::stream::once(async move {
                    FtpStreamChunk::OutputChunk {
                        task_index,
                        output,
                    }
                })
                .boxed()
            }
            functions::FlatTaskProfile::PlaceholderVectorFunction(ftp) => {
                let n = ftp.output_length;
                let score = if n > 0 {
                    rust_decimal::Decimal::ONE / rust_decimal::Decimal::from(n)
                } else {
                    rust_decimal::Decimal::ZERO
                };
                let output = objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(
                    vec![score; n as usize],
                );
                futures::stream::once(async move {
                    FtpStreamChunk::OutputChunk {
                        task_index,
                        output,
                    }
                })
                .boxed()
            }
            functions::FlatTaskProfile::MapPlaceholderVectorFunction(ftp) => {
                let output = objectiveai_sdk::functions::expression::TaskOutputOwned::Vectors(
                    ftp.placeholders
                        .iter()
                        .map(|p| {
                            let n = p.output_length;
                            let score = if n > 0 {
                                rust_decimal::Decimal::ONE / rust_decimal::Decimal::from(n)
                            } else {
                                rust_decimal::Decimal::ZERO
                            };
                            vec![score; n as usize]
                        })
                        .collect(),
                );
                futures::stream::once(async move {
                    FtpStreamChunk::OutputChunk {
                        task_index,
                        output,
                    }
                })
                .boxed()
            }
        }
    }

    fn execute_map_function_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
        ftp: functions::MapFunctionFlatTaskProfile,
        created: u64,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
        swiss_round: Option<u64>,
        swiss_pool_index: Option<u64>,
        split_index: Option<u64>,
    ) -> impl Stream<Item = FtpStreamChunk> + Send + 'static {
        // initialize output and task indices
        let ftp_inner_len = ftp.len();
        let mut task_indices = Vec::with_capacity(ftp_inner_len);
        let mut output = Vec::with_capacity(ftp_inner_len);
        let mut current_task_index = 0;
        for ftp in &ftp.functions {
            task_indices.push(current_task_index);
            current_task_index += ftp.task_index_len() as u64;
            // safety: these should all be replaced without exception
            output.push(
                objectiveai_sdk::functions::expression::TaskOutputOwned::Err {
                    error: serde_json::Value::Null,
                },
            );
        }

        // Combine all mapped instance streams, polling them concurrently.
        // SelectAll polls every contained stream on every outer poll, so
        // the N mapped function instances actually run in parallel.
        // Previously this used `stream::iter(...).flatten()`, which ran
        // each instance to completion before pulling the next.
        let outer_task_indices = task_indices.clone();
        let mut select = futures::stream::SelectAll::new();
        for (i, inner_ftp) in ftp.functions.into_iter().enumerate() {
            select.push(
                self.clone().execute_function_ftp_streaming(
                    ctx.clone(),
                    request.clone(),
                    inner_ftp,
                    None,
                    created,
                    task_index + outer_task_indices[i],
                    choice_indexer.clone(),
                    swiss_round,
                    swiss_pool_index,
                    split_index,
                ).boxed()
            );
        }
        let stream = select;

        // return stream, yielding chunks and updating retry token and output
        async_stream::stream! {
            futures::pin_mut!(stream);
            while let Some(chunk) = stream.next().await {
                match chunk {
                    FtpStreamChunk::FunctionExecutionChunk(chunk) => {
                        yield FtpStreamChunk::FunctionExecutionChunk(chunk);
                    }
                    FtpStreamChunk::OutputChunk {
                        task_index: chunk_task_index,
                        output: chunk_output,
                    } => {
                        // get local index
                        let local_index = task_indices
                            .iter()
                            .position(|&ti| {
                                ti == (chunk_task_index - task_index)
                            })
                            .unwrap();
                        // insert output into correct position
                        output[local_index] = chunk_output;
                    }
                    FtpStreamChunk::VectorCompletionTaskChunk(_) => {
                        unreachable!()
                    }
                }
            }

            // yield final output chunk - collect mapped function sub-outputs
            let collected_output = {
                use objectiveai_sdk::functions::expression::TaskOutputOwned;
                // Determine the type from the first non-error output
                let first_ok = output.iter().find(|o| !matches!(o, TaskOutputOwned::Err { .. }));
                match first_ok {
                    Some(TaskOutputOwned::Scalar(_)) => {
                        // All scalars → Vector
                        TaskOutputOwned::Vector(
                            output.into_iter().map(|o| match o {
                                TaskOutputOwned::Scalar(s) => s,
                                TaskOutputOwned::Err { .. } => rust_decimal::Decimal::ZERO,
                                _ => rust_decimal::Decimal::ZERO,
                            }).collect()
                        )
                    }
                    Some(TaskOutputOwned::Vector(_)) => {
                        // All vectors → Vectors
                        TaskOutputOwned::Vectors(
                            output.into_iter().map(|o| match o {
                                TaskOutputOwned::Vector(v) => v,
                                TaskOutputOwned::Err { .. } => Vec::new(),
                                _ => Vec::new(),
                            }).collect()
                        )
                    }
                    _ => {
                        // All errors or empty
                        TaskOutputOwned::Err { error: serde_json::Value::Null }
                    }
                }
            };
            yield FtpStreamChunk::OutputChunk {
                task_index,
                output: collected_output,
            };
        }
    }

    fn execute_function_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
        ftp: functions::FunctionFlatTaskProfile,
        response_id: Option<String>,
        created: u64,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
        swiss_round: Option<u64>,
        swiss_pool_index: Option<u64>,
        split_index: Option<u64>,
    ) -> impl Stream<Item = FtpStreamChunk> + Send + 'static {
        // identify the completion and get response type
        let response_id = response_id.unwrap_or_else(|| self::response_id(created));
        let object = match ftp.r#type {
            functions::FunctionType::Scalar =>
                objectiveai_sdk::functions::executions::response::streaming::Object::ScalarFunctionExecutionChunk,
            functions::FunctionType::Vector { .. } =>
                objectiveai_sdk::functions::executions::response::streaming::Object::VectorFunctionExecutionChunk,
        };

        // initialize task indices
        let task_indices = ftp.task_indices();

        // extract output expressions from each task for later transformation
        let task_output_expressions: Vec<Option<(objectiveai_sdk::functions::expression::Expression, bool)>> =
            ftp.tasks
                .iter()
                .map(|task| {
                    task.as_ref().and_then(|t| match t {
                        functions::FlatTaskProfile::Function(f) => {
                            f.task_output.clone().map(|expr| (expr, f.invert_output))
                        }
                        functions::FlatTaskProfile::MapFunction(mf) => Some((mf.task_output.clone(), mf.invert_output)),
                        functions::FlatTaskProfile::VectorCompletion(vc) => Some((vc.output.clone(), vc.invert_output)),
                        functions::FlatTaskProfile::MapVectorCompletion(mvc) => Some((mvc.task_output.clone(), mvc.invert_output)),
                        functions::FlatTaskProfile::PlaceholderScalarFunction(p) => Some((p.output.clone(), p.invert_output)),
                        functions::FlatTaskProfile::MapPlaceholderScalarFunction(p) => Some((p.task_output.clone(), p.invert_output)),
                        functions::FlatTaskProfile::PlaceholderVectorFunction(p) => Some((p.output.clone(), p.invert_output)),
                        functions::FlatTaskProfile::MapPlaceholderVectorFunction(p) => Some((p.task_output.clone(), p.invert_output)),
                    })
                })
                .collect();

        // store function input and type for expression evaluation
        let ftp_input = ftp.input.clone();
        let ftp_type = ftp.r#type.clone();

        // initialize output_input (stores validated TaskOutputOwneds directly)
        // and collect errors from task output expressions
        let tasks_len = ftp.tasks.len();
        let mut output_input: Vec<
            Option<objectiveai_sdk::functions::expression::TaskOutputOwned>,
        > = Vec::with_capacity(tasks_len);
        let mut task_output_errors: Vec<super::TaskOutputExpressionError> =
            Vec::new();

        for (i, task) in ftp.tasks.iter().enumerate() {
            if task.as_ref().is_some_and(|task| task.len() == 0) {
                // empty map task - apply output expression to empty result
                let raw_output = match task.as_ref() {
                    Some(functions::FlatTaskProfile::MapFunction(_)) => {
                        objectiveai_sdk::functions::expression::TaskOutputOwned::Vectors(Vec::new())
                    }
                    Some(functions::FlatTaskProfile::MapVectorCompletion(_)) => {
                        objectiveai_sdk::functions::expression::TaskOutputOwned::Vectors(
                            Vec::new(),
                        )
                    }
                    Some(functions::FlatTaskProfile::MapPlaceholderScalarFunction(_))
                    | Some(functions::FlatTaskProfile::MapPlaceholderVectorFunction(_)) => {
                        objectiveai_sdk::functions::expression::TaskOutputOwned::Vectors(Vec::new())
                    }
                    _ => panic!("encountered non-map FlatTaskProfile with length of 0"),
                };
                let (expr, invert_output) = task_output_expressions[i]
                    .as_ref()
                    .expect("empty map task must have output expression");
                let (transformed, error) = apply_task_output_expression(
                    &ftp_input,
                    raw_output,
                    expr,
                    *invert_output,
                    &ftp_type,
                );
                if let Some(err) = error {
                    task_output_errors.push(super::TaskOutputExpressionError {
                        task_index: i,
                        message: err.message.to_string(),
                    });
                    output_input.push(None);
                } else {
                    output_input.push(Some(transformed));
                }
            } else {
                // skipped task or unrun task
                output_input.push(None);
            }
        }

        // create new choice indexer for children
        let child_choice_indexer = Arc::new(ChoiceIndexer::new(0));

        // Combine all sub-task streams, polling them concurrently.
        //
        // Pre-collect into a Vec so that we (a) own the BoxStreams and (b)
        // use SelectAll, which polls every contained stream on every
        // outer poll — this is what makes the sub-tasks of a branch
        // function (e.g. tweet-ranker's three children) actually run in
        // parallel. Previously this used `stream::iter(...).flatten()`,
        // which serialised the sub-tasks: it ran each sub-task to
        // completion before pulling the next one out of the iterator.
        let outer_task_indices = task_indices.clone();
        let mut select = futures::stream::SelectAll::new();
        for (i, inner_ftp) in ftp.tasks.into_iter().enumerate() {
            if let Some(inner_ftp) = inner_ftp {
                if inner_ftp.len() > 0 {
                    select.push(self.clone().execute_ftp_streaming(
                        ctx.clone(),
                        request.clone(),
                        inner_ftp,
                        created,
                        task_index + task_indices[i],
                        child_choice_indexer.clone(),
                        swiss_round,
                        swiss_pool_index,
                        split_index,
                    ));
                }
            }
        }
        let stream = select;
        let task_indices = outer_task_indices;

        // track whether child errors occurred
        let mut tasks_errors = false;

        // track usage
        let mut usage =
            objectiveai_sdk::agent::completions::response::Usage::default();

        // identifiers
        let function = ftp.function_path;
        let profile = ftp.profile_path;

        // return stream, yielding chunks and updating retry token and output
        async_stream::stream! {
            futures::pin_mut!(stream);
            while let Some(chunk) = stream.next().await {
                match chunk {
                    FtpStreamChunk::VectorCompletionTaskChunk(chunk) => {
                        tasks_errors |= chunk.error.is_some() || chunk
                            .inner
                            .completions
                            .iter()
                            .any(|v| v.inner.error.is_some());
                        if let Some(completion_usage) = &chunk.inner.usage {
                            usage.push(completion_usage);
                        }
                        yield FtpStreamChunk::FunctionExecutionChunk(
                            objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionTaskChunk {
                                index: choice_indexer.get(
                                    task_index as usize,
                                ),
                                task_index,
                                task_path: ftp.path.clone(),
                                swiss_round,
                                swiss_pool_index,
                                split_index,
                                inner: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk {
                                    id: response_id.clone(),
                                    tasks: vec![
                                        objectiveai_sdk::functions::executions::response::streaming::TaskChunk::VectorCompletion(
                                            chunk,
                                        ),
                                    ],
                                    tasks_errors: if tasks_errors {
                                        Some(true)
                                    } else {
                                        None
                                    },
                                    reasoning: None,
                                    output: None,
                                    error: None,
                                    created,
                                    function: function.clone(),
                                    profile: profile.clone(),
                                    object,
                                    usage: None,
                                },
                            },
                        );
                    }
                    FtpStreamChunk::FunctionExecutionChunk(chunk) => {
                        tasks_errors |= chunk.inner.error.is_some()
                            || chunk.inner.tasks_errors.unwrap_or(false);
                        if let Some(chunk_usage) = &chunk.inner.usage {
                            usage.push(chunk_usage);
                        }
                        yield FtpStreamChunk::FunctionExecutionChunk(
                            objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionTaskChunk {
                                index: choice_indexer.get(
                                    task_index as usize,
                                ),
                                task_index,
                                task_path: ftp.path.clone(),
                                swiss_round,
                                swiss_pool_index,
                                split_index,
                                inner: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk {
                                    id: response_id.clone(),
                                    tasks: vec![
                                        objectiveai_sdk::functions::executions::response::streaming::TaskChunk::FunctionExecution(
                                            chunk,
                                        ),
                                    ],
                                    tasks_errors: if tasks_errors {
                                        Some(true)
                                    } else {
                                        None
                                    },
                                    reasoning: None,
                                    output: None,
                                    error: None,
                                    created,
                                    function: function.clone(),
                                    profile: profile.clone(),
                                    object,
                                    usage: None,
                                },
                            },
                        );
                    }
                    FtpStreamChunk::OutputChunk {
                        task_index: chunk_task_index,
                        output: chunk_output,
                    } => {
                        // get local index
                        let local_index = task_indices
                            .iter()
                            .position(|&ti| {
                                ti == (chunk_task_index - task_index)
                            })
                            .unwrap();
                        // apply task output expression to transform raw output into TaskOutputOwned
                        // All non-skipped tasks have required output expressions
                        let (expr, invert_output) = task_output_expressions[local_index]
                            .as_ref()
                            .expect("non-skipped task must have output expression");
                        let (transformed_output, transform_error) = apply_task_output_expression(
                            &ftp_input,
                            chunk_output,
                            expr,
                            *invert_output,
                            &ftp_type,
                        );
                        // collect error if any
                        if let Some(err) = transform_error {
                            task_output_errors.push(super::TaskOutputExpressionError {
                                task_index: local_index,
                                message: err.message.to_string(),
                            });
                            // don't store invalid outputs
                        } else {
                            // insert transformed output into correct position
                            output_input[local_index] = Some(transformed_output);
                        }
                    }
                }
            }

            // compute final output as weighted average of task outputs
            let output = compute_weighted_function_output(
                &ftp.r#type,
                &ftp.profile,
                &output_input,
            );

            // build error from task output expression errors if any
            let output_error = if !task_output_errors.is_empty() {
                Some(objectiveai_sdk::error::ResponseError::from(
                    &super::Error::TaskOutputExpressionErrors(task_output_errors),
                ))
            } else {
                None
            };

            // yield final inner function chunk
            yield FtpStreamChunk::FunctionExecutionChunk(
                objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionTaskChunk {
                    index: choice_indexer.get(
                        task_index as usize,
                    ),
                    task_index,
                    task_path: ftp.path,
                    swiss_round,
                    swiss_pool_index,
                    split_index,
                    inner: objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk {
                        id: response_id.clone(),
                        tasks: Vec::new(),
                        tasks_errors: if tasks_errors || output_error.is_some() {
                            Some(true)
                        } else {
                            None
                        },
                        reasoning: None,
                        output: Some(objectiveai_sdk::functions::executions::response::Output { output: output.clone() }),
                        error: output_error,
                        created,
                        function,
                        profile,
                        object,
                        usage: Some(usage),
                    },
                },
            );

            // yield final output chunk
            yield FtpStreamChunk::OutputChunk {
                task_index,
                output,
            };
        }
    }

    async fn execute_map_vector_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
        ftp: functions::MapVectorCompletionFlatTaskProfile,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
    ) -> impl Stream<Item = FtpStreamChunk> + Send + 'static {
        // initialize output (each sub-task produces a scores vector)
        let ftp_inner_len = ftp.vector_completions.len();
        let mut output: Vec<Vec<rust_decimal::Decimal>> = Vec::with_capacity(ftp_inner_len);
        for _ in 0..ftp_inner_len {
            output.push(Vec::new());
        }

        // Combine all mapped vector-completion instance streams, polling
        // them concurrently. `execute_vector_ftp_streaming` is an `async
        // fn` doing HTTP setup, so we must run setup in parallel via
        // `join_all` — otherwise we'd just have moved the serial-flatten
        // bug from streaming-time to setup-time. Once all streams exist,
        // SelectAll polls every contained stream on every outer poll, so
        // the N mapped vector-completion instances actually run in
        // parallel. Previously this used `stream::iter(...).flatten()`,
        // which serialised them.
        let setup_futs = ftp.vector_completions
            .into_iter()
            .enumerate()
            .map(|(i, inner_ftp)| {
                self.clone().execute_vector_ftp_streaming(
                    ctx.clone(),
                    request.clone(),
                    inner_ftp,
                    task_index + i as u64,
                    choice_indexer.clone(),
                )
            });
        let inner_streams = futures::future::join_all(setup_futs).await;
        let mut select = futures::stream::SelectAll::new();
        for inner_stream in inner_streams {
            select.push(inner_stream.boxed());
        }
        let stream = select;

        // return stream, yielding chunks and updating retry token and output
        async_stream::stream! {
            futures::pin_mut!(stream);
            while let Some(chunk) = stream.next().await {
                match chunk {
                    FtpStreamChunk::VectorCompletionTaskChunk(chunk) => {
                        yield FtpStreamChunk::VectorCompletionTaskChunk(chunk);
                    }
                    FtpStreamChunk::OutputChunk {
                        task_index: chunk_task_index,
                        output: chunk_output,
                    } => {
                        // get local index
                        let local_index =
                            (chunk_task_index - task_index) as usize;
                        // insert output into correct position
                        output[local_index] = match chunk_output {
                            objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(scores) => scores,
                            _ => unreachable!(),
                        };
                    }
                    FtpStreamChunk::FunctionExecutionChunk(_) => {
                        unreachable!();
                    }
                }
            }
            // yield final output chunk
            yield FtpStreamChunk::OutputChunk {
                task_index,
                output: objectiveai_sdk::functions::expression::TaskOutputOwned::Vectors(output),
            };
        }
    }

    async fn execute_vector_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
        ftp: functions::VectorCompletionFlatTaskProfile,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
    ) -> impl Stream<Item = FtpStreamChunk> + Send + 'static {
        let request_base = &*request;
        let request_responses_len = ftp.responses.len();
        let mut stream = match self
            .vector_client
            .clone()
            .create_streaming_handle_usage(
                ctx,
                Arc::new(
                    objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams {
                        messages: ftp.messages,
                        provider: request_base.provider.clone(),
                        swarm: objectiveai_sdk::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
                            ftp.swarm.into_base(),
                        ),
                        seed: request_base.seed,
                        stream: request_base.stream,
                        responses: ftp.responses,
                        continuation: request_base.continuation.clone(),
                    },
                ),
            )
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                return futures::future::Either::Left(
                    StreamOnce::new(
                        FtpStreamChunk::VectorCompletionTaskChunk(
                            objectiveai_sdk::functions::executions::response::streaming::VectorCompletionTaskChunk {
                                index: choice_indexer.get(
                                    task_index as usize,
                                ),
                                task_index,
                                task_path: ftp.path.clone(),
                                inner: objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk::default_from_request_responses_len(
                                    request_responses_len,
                                ),
                                error: Some(objectiveai_sdk::error::ResponseError::from(&e))
                            }
                        ),
                    ).chain(StreamOnce::new(
                        FtpStreamChunk::OutputChunk {
                            task_index,
                            output: objectiveai_sdk::functions::expression::TaskOutputOwned::Vector({
                                let n = request_responses_len;
                                vec![
                                    rust_decimal::Decimal::ONE / rust_decimal::Decimal::from(n);
                                    n
                                ]
                            }),
                        }
                    )),
                );
            }
        };

        let mut aggregate: Option<
            objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk,
        > = None;

        futures::future::Either::Right(async_stream::stream! {
            while let Some(chunk) = stream.next().await {
                // push chunk to aggregate
                match &mut aggregate {
                    Some(aggregate) => {
                        aggregate.push(&chunk);
                    }
                    None => {
                        aggregate = Some(chunk.clone());
                    }
                }
                // yield chunk as FunctionResponseChunk
                yield FtpStreamChunk::VectorCompletionTaskChunk(
                    objectiveai_sdk::functions::executions::response::streaming::VectorCompletionTaskChunk {
                        index: choice_indexer.get(
                            task_index as usize,
                        ),
                        task_index,
                        task_path: ftp.path.clone(),
                        inner: chunk,
                        error: None,
                    }
                );
            }
            // unwrap aggregate
            let aggregate = aggregate.unwrap();
            // yield output chunk
            yield FtpStreamChunk::OutputChunk {
                task_index,
                output: objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(aggregate.scores),
            };
        })
    }

    async fn create_reasoning_summary_streaming(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
        description: Option<String>,
        output: objectiveai_sdk::functions::expression::TaskOutputOwned,
        confidence_responses: Vec<ConfidenceResponse>,
    ) -> impl Stream<Item = objectiveai_sdk::functions::executions::response::streaming::ReasoningSummaryChunk>
    + Send
    + 'static{
        // construct the prompt
        let mut parts = Vec::new();
        parts.push(objectiveai_sdk::agent::completions::message::RichContentPart::Text {
            text: match description {
                Some(description) => format!(
                    "The ObjectiveAI Function has the following description: \"{}\"\n\nThe user provided the following input to the ObjectiveAI Function:\n",
                    description,
                ),
                None => "The user provided the following input to an ObjectiveAI Function\n".to_string(),
            },
        });
        parts.extend(request.input.clone().to_rich_content_parts(0));
        parts.push(objectiveai_sdk::agent::completions::message::RichContentPart::Text {
            text: match output {
                objectiveai_sdk::functions::expression::TaskOutputOwned::Scalar(scalar) => {
                    format!(
                        "\n\nThe ObjectiveAI Function produced the following score: {}%\n\n",
                        (scalar * rust_decimal::dec!(100)).round_dp(2),
                    )
                },
                objectiveai_sdk::functions::expression::TaskOutputOwned::Vector(vector) => {
                    format!(
                        "\n\nThe ObjectiveAI Function produced the following vector of scores: [{}]\n\n",
                        vector.iter()
                            .map(|v| {
                                format!(
                                    "{}%",
                                    (v * rust_decimal::dec!(100)).round_dp(2),
                                )
                            })
                            .collect::<Vec<String>>()
                            .join(", ")
                    )
                },
                objectiveai_sdk::functions::expression::TaskOutputOwned::Vectors(vectors) => {
                    let formatted: Vec<String> = vectors.iter().map(|vector| {
                        format!("[{}]", vector.iter()
                            .map(|v| format!("{}%", (v * rust_decimal::dec!(100)).round_dp(2)))
                            .collect::<Vec<String>>()
                            .join(", "))
                    }).collect();
                    format!(
                        "\n\nThe ObjectiveAI Function produced the following vectors of scores: [{}]\n\n",
                        formatted.join(", ")
                    )
                },
                objectiveai_sdk::functions::expression::TaskOutputOwned::Err {
                    error: serde_json::Value::Number(n),
                } if {
                    n.as_f64().is_some()
                        && n.as_f64().unwrap() >= 0.0
                        && n.as_f64().unwrap() <= 1.0
                } => format!(
                    "\n\nThe ObjectiveAI Function erroneously produced the following score: {:.2}%\n\n",
                    n.as_f64().unwrap() * 100.0,
                ),
                objectiveai_sdk::functions::expression::TaskOutputOwned::Err {
                    error: serde_json::Value::Array(arr),
                } if {
                    arr
                        .iter()
                        .all(|v| v.as_f64().is_some())
                    && {
                        let sum: f64 = arr
                            .iter()
                            .map(|v| v.as_f64().unwrap())
                            .sum();
                        sum >= 0.99 && sum <= 1.01
                    }
                } => format!(
                    "\n\nThe ObjectiveAI Function erroneously produced the following vector of scores: [{}]\n\n",
                    arr.iter()
                        .map(|v| format!("{:.2}%", v.as_f64().unwrap() * 100.0))
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
                objectiveai_sdk::functions::expression::TaskOutputOwned::Err { error } => format!(
                    "\n\nThe ObjectiveAI Function erroneously produced the following output:\n{}\n\n",
                    serde_json::to_string_pretty(&error).unwrap(),
                ),
            }
        });
        parts.push(objectiveai_sdk::agent::completions::message::RichContentPart::Text {
            text: "The ObjectiveAI Function used LLM Swarms to arrive at this output by making assertions with associated confidence scores:\n\n".to_string(),
        });
        parts.extend(ConfidenceResponse::assertions(confidence_responses));
        parts.push(objectiveai_sdk::agent::completions::message::RichContentPart::Text {
            text: "\n\nYou are to present the output and summarize the reasoning process used by the ObjectiveAI Function to arrive at the output based on the assertions made above. Focus on the most confident assertions and explain how they contributed to the final output. If there were any low-confidence assertions, mention them with the caveat of low confidence. Provide a clear summary of the overall reasoning process.".to_string(),
        });

        // create the streaming agent completion
        let mut stream = match self
            .agent_client
            .clone()
            .create_streaming_handle_usage(
                ctx,
                Arc::new(
                    objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
                        messages: vec![objectiveai_sdk::agent::completions::message::Message::User(
                            objectiveai_sdk::agent::completions::message::UserMessage {
                                content:
                                    objectiveai_sdk::agent::completions::message::RichContent::Parts(
                                        parts,
                                    ),
                                name: None,
                            },
                        )],
                        provider: request.provider.clone(),
                        agent,
                        response_format: None,
                        seed: request.seed,
                        stream: Some(true),
                        continuation: request.continuation.clone(),
                    },
                ),
                None,
                None, // disable_tools
                vec![], // extra_mcp_servers
                indexmap::IndexMap::new(), // extra_mcp_headers
                None,
            )
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                return futures::future::Either::Left(StreamOnce::new(
                    objectiveai_sdk::functions::executions::response::streaming::ReasoningSummaryChunk {
                        inner: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk::default(),
                        error: Some(objectiveai_sdk::error::ResponseError::from(&e)),
                    }
                ));
            }
        };

        // get the first chunk from the stream
        let mut next_agent_chunk = match stream.next().await {
            Some(crate::agent::completions::StreamItem::Chunk(chunk)) => Some(chunk),
            Some(crate::agent::completions::StreamItem::State(_)) => {
                // skip state items, try next
                loop {
                    match stream.next().await {
                        Some(crate::agent::completions::StreamItem::Chunk(chunk)) => break Some(chunk),
                        Some(crate::agent::completions::StreamItem::State(_)) => continue,
                        None => break None,
                    }
                }
            }
            None => {
                // agent client will always yield at least one chunk
                unreachable!()
            }
        };

        // stream, buffered by 1 so as to attach errors from chunk.error
        futures::future::Either::Right(async_stream::stream! {
            while let Some(agent_chunk) = next_agent_chunk.take() {
                // fetch the next agent chunk
                let error = loop {
                    match stream.next().await {
                        Some(crate::agent::completions::StreamItem::Chunk(ncc)) => {
                            // check if the current chunk had an error
                            let err = ncc.error.clone();
                            next_agent_chunk = Some(ncc);
                            break err;
                        }
                        Some(crate::agent::completions::StreamItem::State(_)) => {
                            // skip state items
                            continue;
                        }
                        None => {
                            // end the loop after this iteration
                            break None;
                        }
                    }
                };

                // yield the reasoning summary chunk
                yield objectiveai_sdk::functions::executions::response::streaming::ReasoningSummaryChunk {
                    inner: agent_chunk,
                    error,
                };
            }
        })
    }
}

/// Internal chunk type for streaming execution.
///
/// Represents different kinds of chunks produced during flattened task
/// profile execution.
#[derive(Debug, Clone)]
enum FtpStreamChunk {
    /// A chunk from a Vector Completion task.
    VectorCompletionTaskChunk(
        objectiveai_sdk::functions::executions::response::streaming::VectorCompletionTaskChunk,
    ),
    /// A chunk from a nested Function execution.
    FunctionExecutionChunk(
        objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionTaskChunk,
    ),
    /// The final output of a task.
    OutputChunk {
        /// Index of the task in the flattened structure.
        task_index: u64,
        /// The computed output of the task.
        output: objectiveai_sdk::functions::expression::TaskOutputOwned,
    },
}

/// A response option with its aggregated confidence for reasoning summaries.
///
/// Tracks confidence scores and reasoning across multiple Vector Completion
/// tasks that share the same response option.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfidenceResponse {
    /// Hash of the response for deduplication.
    #[serde(skip)]
    pub response_hash: u64,
    /// Task paths that included this response.
    #[serde(skip)]
    pub paths: Vec<Vec<u64>>,
    /// Number of times this response appeared (for normalization).
    #[serde(skip)]
    pub confidence_count: rust_decimal::Decimal,

    /// The response content.
    pub response: objectiveai_sdk::agent::completions::message::RichContent,
    /// Aggregated confidence score.
    pub confidence: rust_decimal::Decimal,
    /// Collected reasoning from LLMs that voted for this response.
    pub reasoning: Vec<String>,
}

impl ConfidenceResponse {
    /// Formats all confidence responses as assertion parts for the reasoning prompt.
    pub fn assertions(
        confidence_responses: Vec<ConfidenceResponse>,
    ) -> impl Iterator<Item = objectiveai_sdk::agent::completions::message::RichContentPart>
    {
        confidence_responses
            .into_iter()
            .flat_map(ConfidenceResponse::assertion)
    }

    /// Formats this confidence response as JSON assertion parts.
    pub fn assertion(
        self,
    ) -> impl Iterator<Item = objectiveai_sdk::agent::completions::message::RichContentPart>
    {
        if self.confidence < rust_decimal::dec!(0.00005) {
            return None.into_iter().flatten();
        }
        Some(
            std::iter::once(objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                text: "{\n    \"assertion\": \"".to_string(),
            })
            .chain({
                enum Iter<P> {
                    Text(Option<String>),
                    Parts(P),
                }
                impl<P: Iterator<Item = objectiveai_sdk::agent::completions::message::RichContentPart>>
                    Iterator for Iter<P>
                {
                    type Item = objectiveai_sdk::agent::completions::message::RichContentPart;
                    fn next(&mut self) -> Option<Self::Item> {
                        match self {
                        Iter::Text(opt_text) => {
                            opt_text.take().map(|text| {
                                objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                                    text,
                                }
                            })
                        }
                        Iter::Parts(parts_iter) => parts_iter.next(),
                    }
                    }
                }
                match self.response {
                    objectiveai_sdk::agent::completions::message::RichContent::Text(text) => {
                        Iter::Text(Some(
                            json_escape::escape_str(&text).to_string(),
                        ))
                    }
                    objectiveai_sdk::agent::completions::message::RichContent::Parts(rich_parts) => {
                        Iter::Parts(rich_parts.into_iter().map(|part| {
                            if let objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                            text,
                        } = part {
                            objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                                text: json_escape::escape_str(&text)
                                    .to_string(),
                            }
                        } else {
                            part
                        }
                        }))
                    }
                }
            })
            .chain(std::iter::once(
                objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                    text: format!(
                        "\",\n    \"confidence\": \"{}%\"",
                        (self.confidence * rust_decimal::dec!(100)).round_dp(2),
                    ),
                },
            ))
            .chain(std::iter::once(
                objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                    text: if self.reasoning.is_empty() {
                        "\n}".to_string()
                    } else {
                        format!(
                            ",\n    \"reasoning\": [{}]\n}}",
                            self.reasoning
                                .into_iter()
                                .map(|r| format!(
                                    "\"{}\"",
                                    json_escape::escape_str(&r)
                                ))
                                .collect::<Vec<String>>()
                                .join(", ")
                        )
                    },
                },
            )),
        )
        .into_iter()
        .flatten()
    }
}

#[cfg(test)]
mod invert_output_tests {
    use super::*;
    use objectiveai_sdk::functions::expression::{
        Expression, TaskOutputOwned,
    };
    use rust_decimal::dec;

    fn empty_input() -> objectiveai_sdk::functions::expression::InputValue {
        objectiveai_sdk::functions::expression::InputValue::Object(
            indexmap::IndexMap::new(),
        )
    }

    #[test]
    fn invert_task_output_scalar() {
        let input = empty_input();
        let raw = TaskOutputOwned::Scalar(dec!(0.75));
        let expr = Expression::Starlark("output".to_string());
        let (out, err) = apply_task_output_expression(
            &input,
            raw,
            &expr,
            true,
            &functions::FunctionType::Scalar,
        );
        assert!(err.is_none());
        match out {
            TaskOutputOwned::Scalar(v) => assert_eq!(v, dec!(0.25)),
            other => panic!("expected scalar output, got {:?}", other),
        }
    }

    #[test]
    fn invert_task_output_vector() {
        let input = empty_input();
        let raw = TaskOutputOwned::Vector(vec![
            dec!(0.75),
            dec!(0.25),
            dec!(0.0),
        ]);
        let expr = Expression::Starlark("output".to_string());
        let (out, err) = apply_task_output_expression(
            &input,
            raw,
            &expr,
            true,
            &functions::FunctionType::Vector {
                output_length: None,
                input_split: None,
                input_merge: None,
            },
        );
        assert!(err.is_none());
        match out {
            TaskOutputOwned::Vector(v) => {
                assert_eq!(v, vec![dec!(0.125), dec!(0.375), dec!(0.5)])
            }
            other => panic!("expected vector output, got {:?}", other),
        }
    }

    #[test]
    fn invert_task_output_vector_scores() {
        let input = empty_input();
        let raw = TaskOutputOwned::Vector(vec![dec!(0.75), dec!(0.25), dec!(0.0)]);
        let expr = Expression::Starlark("output".to_string());
        let (out, err) = apply_task_output_expression(
            &input,
            raw,
            &expr,
            true,
            &functions::FunctionType::Vector {
                output_length: None,
                input_split: None,
                input_merge: None,
            },
        );
        assert!(err.is_none());
        match out {
            TaskOutputOwned::Vector(v) => {
                assert_eq!(v, vec![dec!(0.125), dec!(0.375), dec!(0.5)])
            }
            other => panic!("expected vector output, got {:?}", other),
        }
    }
}
