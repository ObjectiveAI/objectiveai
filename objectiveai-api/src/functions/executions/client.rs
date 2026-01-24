//! Function execution client.

use crate::{
    chat, ctx, functions,
    util::{ChoiceIndexer, StreamOnce},
    vector,
};
use futures::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hasher, sync::Arc, time};

/// Generates a unique response ID for scalar Function executions.
pub fn scalar_response_id(created: u64) -> String {
    let uuid = uuid::Uuid::new_v4();
    format!("sclfnc-{}-{}", uuid.simple(), created)
}

/// Generates a unique response ID for vector Function executions.
pub fn vector_response_id(created: u64) -> String {
    let uuid = uuid::Uuid::new_v4();
    format!("vctfnc-{}-{}", uuid.simple(), created)
}

/// Client for executing Functions.
///
/// Orchestrates Function execution by flattening the Function and Profile
/// into executable tasks and running them (Vector Completions or nested
/// Functions) with streaming output support.
pub struct Client<
    CTXEXT,
    FENSLLM,
    CUSG,
    FENS,
    FVVOTE,
    FCVOTE,
    VUSG,
    FFN,
    FPFL,
    FUSG,
> {
    /// Chat completions client for reasoning summaries.
    pub chat_client: Arc<chat::completions::Client<CTXEXT, FENSLLM, CUSG>>,
    /// Fetcher for Ensemble definitions.
    pub ensemble_fetcher: Arc<
        crate::ensemble::fetcher::CachingFetcher<CTXEXT, FENS>,
    >,
    /// Vector completions client for executing Vector Completion tasks.
    pub vector_client: Arc<
        vector::completions::Client<
            CTXEXT,
            FENSLLM,
            CUSG,
            FENS,
            FVVOTE,
            FCVOTE,
            VUSG,
        >,
    >,
    /// Fetcher for Function definitions.
    pub function_fetcher: Arc<FFN>,
    /// Fetcher for Profile definitions.
    pub profile_fetcher: Arc<FPFL>,
    /// Handler for recording usage after execution.
    pub usage_handler: Arc<FUSG>,
}

impl<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG, FFN, FPFL, FUSG>
    Client<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG, FFN, FPFL, FUSG>
{
    /// Creates a new Function execution client.
    pub fn new(
        chat_client: Arc<chat::completions::Client<CTXEXT, FENSLLM, CUSG>>,
        ensemble_fetcher: Arc<
            crate::ensemble::fetcher::CachingFetcher<CTXEXT, FENS>,
        >,
        vector_client: Arc<
            vector::completions::Client<
                CTXEXT,
                FENSLLM,
                CUSG,
                FENS,
                FVVOTE,
                FCVOTE,
                VUSG,
            >,
        >,
        function_fetcher: Arc<FFN>,
        profile_fetcher: Arc<FPFL>,
        usage_handler: Arc<FUSG>,
    ) -> Self {
        Self {
            chat_client,
            ensemble_fetcher,
            vector_client,
            function_fetcher,
            profile_fetcher,
            usage_handler,
        }
    }
}

impl<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG, FFN, FPFL, FUSG>
    Client<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG, FFN, FPFL, FUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    FENSLLM: crate::ensemble_llm::fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    CUSG: chat::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    FENS: crate::ensemble::fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    FVVOTE: vector::completions::completion_votes_fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    FCVOTE: vector::completions::cache_vote_fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    VUSG: vector::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    FFN: functions::function_fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    FPFL: functions::profile_fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    FUSG: super::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
{
    /// Executes a Function and returns the complete response.
    ///
    /// Collects the full streaming response and records usage.
    pub async fn create_unary_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
    ) -> Result<
        objectiveai::functions::executions::response::unary::FunctionExecution,
        super::Error,
    > {
        let mut aggregate: Option<
            objectiveai::functions::executions::response::streaming::FunctionExecutionChunk,
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
    pub async fn create_streaming_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
    ) -> Result<
        impl Stream<Item = objectiveai::functions::executions::response::streaming::FunctionExecutionChunk>
        + Send
        + Unpin
        + 'static,
        super::Error,
    >{
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut aggregate: Option<
                objectiveai::functions::executions::response::streaming::FunctionExecutionChunk,
            > = None;
            let mut any_usage = false;
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
            while let Some(chunk) = stream.next().await {
                any_usage |= chunk.any_usage();
                match &mut aggregate {
                    Some(aggregate) => aggregate.push(&chunk),
                    None => aggregate = Some(chunk.clone()),
                }
                let _ = tx.send(Ok(chunk));
            }
            drop(stream);
            drop(tx);
            if any_usage {
                self.usage_handler
                    .handle_usage(ctx, request, aggregate.unwrap().into())
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

impl<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG, FFN, FPFL, FUSG>
    Client<CTXEXT, FENSLLM, CUSG, FENS, FVVOTE, FCVOTE, VUSG, FFN, FPFL, FUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    FENSLLM: crate::ensemble_llm::fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    CUSG: chat::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    FENS: crate::ensemble::fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    FVVOTE: vector::completions::completion_votes_fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    FCVOTE: vector::completions::cache_vote_fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    VUSG: vector::completions::usage_handler::UsageHandler<CTXEXT>
        + Send
        + Sync
        + 'static,
    FFN: functions::function_fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    FPFL: functions::profile_fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    FUSG: Send + Sync + 'static,
{
    /// Executes a Function with streaming output.
    ///
    /// Fetches the Function and Profile, flattens them into tasks, and
    /// executes all tasks with streaming output. Handles reasoning summaries
    /// if requested.
    pub async fn create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
    ) -> Result<
        impl Stream<Item = objectiveai::functions::executions::response::streaming::FunctionExecutionChunk>
        + Send
        + 'static,
        super::Error,
    >{
        // timestamp the completion
        let created = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // parse retry token if provided
        let retry_token = request
            .base()
            .retry_token
            .as_ref()
            .map(|token_str| {
                objectiveai::functions::executions::RetryToken::try_from_string(
                    token_str,
                )
                .ok_or(super::Error::InvalidRetryToken)
            })
            .transpose()?
            .map(Arc::new);

        // fetch function flat task profile + latest function/profile versions if publishing
        let mut ftp = self
            .fetch_function_flat_task_profile(ctx.clone(), request.clone())
            .await?;

        // take description from ftp
        let description = ftp.description.take();

        // reasonong data
        let reasoning = request.base().reasoning.is_some();
        let mut reasoning_data = if reasoning {
            Some((
                HashMap::<
                    String,
                    objectiveai::functions::executions::response::streaming::VectorCompletionTaskChunk,
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
                None::<objectiveai::functions::executions::response::streaming::FunctionExecutionChunk>,
            ))
        } else {
            None
        };

        // get function stream
        let stream = self
            .clone()
            .execute_function_ftp_streaming(
                ctx.clone(),
                request.clone(),
                retry_token,
                ftp,
                created,
                0,
                Arc::new(ChoiceIndexer::new(0)),
            )
            .await;

        Ok(async_stream::stream! {
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
                let objectiveai::functions::executions::request::Reasoning {
                    model,
                    models,
                } = request.base().reasoning.as_ref().unwrap();
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
                            let delta = &mut completion
                                .inner
                                .choices[0]
                                .delta;
                            if let Some(reasoning) = delta.reasoning.take() {
                                confidence_response.reasoning.push(reasoning);
                            }
                            if let Some(content) = delta.content.take()
                                && let Ok(vector::completions::ResponseKey {
                                    _think: Some(reasoning),
                                    ..
                                }) = serde_json::from_str(&content)
                            {
                                confidence_response.reasoning.push(reasoning);
                            }
                            if let Some(tool_calls) = delta.tool_calls.take() {
                                for tool_call in tool_calls {
                                    if let objectiveai::chat::completions::response::streaming::ToolCall {
                                        function: Some(
                                            objectiveai::chat::completions::response::streaming::ToolCallFunction {
                                                arguments: Some(arguments),
                                                ..
                                            }
                                        ),
                                        ..
                                    } = tool_call
                                        && let Ok(vector::completions::ResponseKey {
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
                    model.clone(),
                    models.clone(),
                    description,
                    final_chunk.output.clone().expect("missing output"),
                    confidence_responses,
                ).await;

                // yield chunks
                futures::pin_mut!(stream);
                while let Some(chunk) = stream.next().await {
                    // collect usage
                    if let Some(chunk_usage) = &chunk.inner.usage {
                        if let Some(usage) = &mut final_chunk.usage {
                            usage.push_chat_completion_usage(chunk_usage);
                        } else {
                            let mut usage = objectiveai::vector::completions::response::Usage::default();
                            usage.push_chat_completion_usage(chunk_usage);
                            final_chunk.usage = Some(usage);
                        }
                    }

                    // yield chunk
                    yield objectiveai::functions::executions::response::streaming::FunctionExecutionChunk {
                        id: final_chunk.id.clone(),
                        tasks: Vec::new(),
                        tasks_errors: final_chunk.tasks_errors,
                        reasoning: Some(chunk),
                        output: None,
                        error: None,
                        retry_token: None,
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
        })
    }

    async fn fetch_function_flat_task_profile(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
    ) -> Result<functions::FunctionFlatTaskProfile, super::Error> {
        match &*request {
            objectiveai::functions::executions::request::Request::FunctionInlineProfileInline {
                body,
            } => {
                functions::get_flat_task_profile(
                    ctx,
                    Vec::new(),
                    functions::FunctionParam::FetchedOrInline {
                        full_id: None,
                        function: objectiveai::functions::Function::Inline(
                            body.function.clone(),
                        ),
                    },
                    functions::ProfileParam::FetchedOrInline {
                        full_id: None,
                        profile: objectiveai::functions::Profile::Inline(
                            body.profile.clone(),
                        ),
                    },
                    body.base.input.clone(),
                    self.function_fetcher.clone(),
                    self.profile_fetcher.clone(),
                    self.ensemble_fetcher.clone(),
                )
                .await
            }
            objectiveai::functions::executions::request::Request::FunctionInlineProfileRemote {
                path,
                body,
            } => {
                functions::get_flat_task_profile(
                    ctx,
                    Vec::new(),
                    functions::FunctionParam::FetchedOrInline {
                        full_id: None,
                        function: objectiveai::functions::Function::Inline(
                            body.function.clone(),
                        ),
                    },
                    functions::ProfileParam::Remote {
                        owner: path.powner.clone(),
                        repository: path.prepository.clone(),
                        commit: path.pcommit.clone(),
                    },
                    body.base.input.clone(),
                    self.function_fetcher.clone(),
                    self.profile_fetcher.clone(),
                    self.ensemble_fetcher.clone(),
                )
                .await
            }
            objectiveai::functions::executions::request::Request::FunctionRemoteProfileInline {
                path,
                body,
            } => {
                functions::get_flat_task_profile(
                    ctx,
                    Vec::new(),
                    functions::FunctionParam::Remote {
                        owner: path.fowner.clone(),
                        repository: path.frepository.clone(),
                        commit: path.fcommit.clone(),
                    },
                    functions::ProfileParam::FetchedOrInline {
                        full_id: None,
                        profile: objectiveai::functions::Profile::Inline(
                            body.profile.clone(),
                        ),
                    },
                    body.base.input.clone(),
                    self.function_fetcher.clone(),
                    self.profile_fetcher.clone(),
                    self.ensemble_fetcher.clone(),
                )
                .await
            }
            objectiveai::functions::executions::request::Request::FunctionRemoteProfileRemote {
                path,
                body
            } => {
                functions::get_flat_task_profile(
                    ctx,
                    Vec::new(),
                    functions::FunctionParam::Remote {
                        owner: path.fowner.clone(),
                        repository: path.frepository.clone(),
                        commit: path.fcommit.clone(),
                    },
                    functions::ProfileParam::Remote {
                        owner: path.powner.clone(),
                        repository: path.prepository.clone(),
                        commit: path.pcommit.clone(),
                    },
                    body.input.clone(),
                    self.function_fetcher.clone(),
                    self.profile_fetcher.clone(),
                    self.ensemble_fetcher.clone(),
                )
                .await
            }
        }
    }

    fn execute_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
        root_retry_token: Option<
            Arc<objectiveai::functions::executions::RetryToken>,
        >,
        ftp: functions::FlatTaskProfile,
        created: u64,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
    ) -> futures::stream::BoxStream<'static, FtpStreamChunk> {
        match ftp {
            functions::FlatTaskProfile::Function(function_ftp) => {
                futures::stream::once(
                    self.clone().execute_function_ftp_streaming(
                        ctx,
                        request,
                        root_retry_token,
                        function_ftp,
                        created,
                        task_index,
                        choice_indexer,
                    ),
                )
                .flatten()
                .boxed()
            }
            functions::FlatTaskProfile::MapFunction(map_function_ftp) => {
                futures::stream::once(
                    self.clone().execute_map_function_ftp_streaming(
                        ctx,
                        request,
                        root_retry_token,
                        map_function_ftp,
                        created,
                        task_index,
                        choice_indexer,
                    ),
                )
                .flatten()
                .boxed()
            }
            functions::FlatTaskProfile::VectorCompletion(vector_ftp) => {
                futures::stream::once(
                    self.clone().execute_vector_ftp_streaming(
                        ctx,
                        request,
                        root_retry_token,
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
                        root_retry_token,
                        map_vector_ftp,
                        task_index,
                        choice_indexer,
                    ),
                )
                .flatten()
                .boxed()
            }
        }
    }

    async fn execute_map_function_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
        root_retry_token: Option<
            Arc<objectiveai::functions::executions::RetryToken>,
        >,
        ftp: functions::MapFunctionFlatTaskProfile,
        created: u64,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
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
                objectiveai::functions::expression::FunctionOutput::Err(
                    serde_json::Value::Null,
                ),
            );
        }

        // initialize retry token
        let ftp_task_index_len = ftp.task_index_len();
        let mut retry_token = objectiveai::functions::executions::RetryToken(
            Vec::with_capacity(ftp_task_index_len),
        );
        for _ in 0..ftp_task_index_len {
            retry_token.0.push(None);
        }

        // combine all streams into one
        let outer_task_indices = task_indices.clone();
        let stream = futures::stream::iter(
            ftp.functions.into_iter().enumerate().map(move |(i, ftp)| {
                futures::stream::once(
                    self.clone().execute_function_ftp_streaming(
                        ctx.clone(),
                        request.clone(),
                        root_retry_token.clone(),
                        ftp,
                        created,
                        task_index + outer_task_indices[i],
                        choice_indexer.clone(),
                    ),
                )
                .flatten()
            }),
        )
        .flatten();

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
                        retry_token: chunk_retry_token,
                    } => {
                        // get local index
                        let local_index = task_indices
                            .iter()
                            .position(|&ti| {
                                ti == (chunk_task_index - task_index)
                            })
                            .unwrap();
                        // insert retry token into correct position
                        retry_token.insert(local_index, chunk_retry_token);
                        // insert output into correct position
                        output[local_index] = match chunk_output {
                            objectiveai::functions::expression::TaskOutputOwned::Function(output) => output,
                            _ => unreachable!(),
                        };
                    }
                    FtpStreamChunk::VectorCompletionTaskChunk(_) => {
                        unreachable!()
                    }
                }
            }

            // yield final output chunk
            yield FtpStreamChunk::OutputChunk {
                task_index,
                output: objectiveai::functions::expression::TaskOutputOwned::MapFunction(output),
                retry_token,
            };
        }
    }

    async fn execute_function_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
        root_retry_token: Option<
            Arc<objectiveai::functions::executions::RetryToken>,
        >,
        ftp: functions::FunctionFlatTaskProfile,
        created: u64,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
    ) -> impl Stream<Item = FtpStreamChunk> + Send + 'static {
        // identify the completion and get response type
        let (response_id, object) = match ftp.r#type {
            functions::FunctionType::Scalar => (
                scalar_response_id(created),
                objectiveai::functions::executions::response::streaming::Object::ScalarFunctionExecutionChunk,
            ),
            functions::FunctionType::Vector { .. } => (
                vector_response_id(created),
                objectiveai::functions::executions::response::streaming::Object::VectorFunctionExecutionChunk,
            ),
        };

        // initialize task indices
        let task_indices = ftp.task_indices();

        // initialize output_input
        let tasks_len = ftp.tasks.len();
        let mut output_input = Vec::with_capacity(tasks_len);
        for task in &ftp.tasks {
            output_input.push(
                if task.as_ref().is_some_and(|task| task.len() == 0) {
                    // empty map task
                    match task.as_ref() {
                        Some(functions::FlatTaskProfile::MapFunction(_)) => {
                            Some(objectiveai::functions::expression::TaskOutput::Owned(
                                objectiveai::functions::expression::TaskOutputOwned::MapFunction(Vec::new()),
                            ))
                        }
                        Some(functions::FlatTaskProfile::MapVectorCompletion(_)) => {
                            Some(objectiveai::functions::expression::TaskOutput::Owned(
                                objectiveai::functions::expression::TaskOutputOwned::MapVectorCompletion(
                                    Vec::new(),
                                ),
                            ))
                        }
                        _ => panic!("encountered non-map FlatTaskProfile with length of 0"),
                    }
                } else {
                    // skipped task or unrun task
                    None
                },
            );
        }

        // initialize retry token
        let ftp_task_index_len = ftp.task_index_len();
        let mut retry_token = objectiveai::functions::executions::RetryToken(
            Vec::with_capacity(ftp_task_index_len),
        );
        for _ in 0..ftp_task_index_len {
            retry_token.0.push(None);
        }

        // create new choice indexer for children
        let child_choice_indexer = Arc::new(ChoiceIndexer::new(0));

        // combine all streams into one
        let outer_task_indices = task_indices.clone();
        let stream = futures::stream::iter(
            ftp.tasks.into_iter().enumerate().filter_map(
                move |(i, inner_ftp)| {
                    inner_ftp
                        .map(|inner_ftp| {
                            if inner_ftp.len() > 0 {
                                Some(self.clone().execute_ftp_streaming(
                                    ctx.clone(),
                                    request.clone(),
                                    root_retry_token.clone(),
                                    inner_ftp,
                                    created,
                                    task_index + task_indices[i],
                                    child_choice_indexer.clone(),
                                ))
                            } else {
                                None
                            }
                        })
                        .flatten()
                },
            ),
        )
        .flatten();
        let task_indices = outer_task_indices;

        // track whether child errors occurred
        let mut tasks_errors = false;

        // track usage
        let mut usage =
            objectiveai::vector::completions::response::Usage::default();

        // identifiers
        let function =
            ftp.full_function_id.map(|(owner, repository, commit)| {
                format!("{}/{}/{}", owner, repository, commit)
            });
        let profile = ftp.full_profile_id.map(|(owner, repository, commit)| {
            format!("{}/{}/{}", owner, repository, commit)
        });

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
                            .any(|v| v.error.is_some());
                        if let Some(completion_usage) = &chunk.inner.usage {
                            usage.push(completion_usage);
                        }
                        yield FtpStreamChunk::FunctionExecutionChunk(
                            objectiveai::functions::executions::response::streaming::FunctionExecutionTaskChunk {
                                index: choice_indexer.get(
                                    task_index as usize,
                                ),
                                task_index,
                                task_path: ftp.path.clone(),
                                inner: objectiveai::functions::executions::response::streaming::FunctionExecutionChunk {
                                    id: response_id.clone(),
                                    tasks: vec![
                                        objectiveai::functions::executions::response::streaming::TaskChunk::VectorCompletion(
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
                                    retry_token: None,
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
                            objectiveai::functions::executions::response::streaming::FunctionExecutionTaskChunk {
                                index: choice_indexer.get(
                                    task_index as usize,
                                ),
                                task_index,
                                task_path: ftp.path.clone(),
                                inner: objectiveai::functions::executions::response::streaming::FunctionExecutionChunk {
                                    id: response_id.clone(),
                                    tasks: vec![
                                        objectiveai::functions::executions::response::streaming::TaskChunk::FunctionExecution(
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
                                    retry_token: None,
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
                        retry_token: chunk_retry_token,
                    } => {
                        // get local index
                        let local_index = task_indices
                            .iter()
                            .position(|&ti| {
                                ti == (chunk_task_index - task_index)
                            })
                            .unwrap();
                        // insert retry token into correct position
                        retry_token.insert(local_index, chunk_retry_token);
                        // insert output into correct position
                        output_input[local_index] = Some(
                            objectiveai::functions::expression::TaskOutput::Owned(chunk_output),
                        );
                    }
                }
            }

            // compile final output
            let params = objectiveai::functions::expression::Params::Ref(
                objectiveai::functions::expression::ParamsRef {
                    input: &ftp.input,
                    tasks: &output_input,
                    map: None,
                },
            );
            let (output, output_error): (
                objectiveai::functions::expression::FunctionOutput,
                Option<objectiveai::error::ResponseError>,
            ) = match (
                ftp.r#type,
                ftp.output.compile_one(&params),
            ) {
                (
                    functions::FunctionType::Scalar,
                    Ok(objectiveai::functions::expression::FunctionOutput::Scalar(scalar)),
                ) if {
                    scalar >= rust_decimal::Decimal::ZERO &&
                        scalar <= rust_decimal::Decimal::ONE
                } => (
                    objectiveai::functions::expression::FunctionOutput::Scalar(scalar),
                    None,
                ),
                (
                    functions::FunctionType::Scalar,
                    Ok(output)
                ) => (
                    output.into_err(),
                    Some(objectiveai::error::ResponseError::from(
                        &super::Error::InvalidScalarOutput,
                    )),
                ),
                (
                    functions::FunctionType::Vector { output_length },
                    Ok(objectiveai::functions::expression::FunctionOutput::Vector(vector)),
                ) if {
                    output_length.is_none_or(|len| len == vector.len() as u64)
                        && {
                            let sum: rust_decimal::Decimal =
                                vector.iter().cloned().sum();
                            sum >= rust_decimal::dec!(0.99) &&
                                sum <= rust_decimal::dec!(1.01)
                        }
                } => (
                    objectiveai::functions::expression::FunctionOutput::Vector(vector),
                    None,
                ),
                (
                    functions::FunctionType::Vector { output_length },
                    Ok(output)
                ) => (
                    output.into_err(),
                    Some(objectiveai::error::ResponseError::from(
                        &super::Error::InvalidVectorOutput(
                            output_length.unwrap_or_default() as usize,
                        ),
                    )),
                ),
                (_, Err(e)) => (
                    objectiveai::functions::expression::FunctionOutput::Err(serde_json::Value::Null),
                    Some(objectiveai::error::ResponseError::from(&super::Error::from(e))),
                ),
            };

            // yield final inner function chunk
            yield FtpStreamChunk::FunctionExecutionChunk(
                objectiveai::functions::executions::response::streaming::FunctionExecutionTaskChunk {
                    index: choice_indexer.get(
                        task_index as usize,
                    ),
                    task_index,
                    task_path: ftp.path,
                    inner: objectiveai::functions::executions::response::streaming::FunctionExecutionChunk {
                        id: response_id.clone(),
                        tasks: Vec::new(),
                        tasks_errors: if tasks_errors {
                            Some(true)
                        } else {
                            None
                        },
                        reasoning: None,
                        output: Some(output.clone()),
                        error: output_error,
                        retry_token: Some(retry_token.to_string()),
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
                output: objectiveai::functions::expression::TaskOutputOwned::Function(output),
                retry_token,
            };
        }
    }

    async fn execute_map_vector_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
        root_retry_token: Option<
            Arc<objectiveai::functions::executions::RetryToken>,
        >,
        ftp: functions::MapVectorCompletionFlatTaskProfile,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
    ) -> impl Stream<Item = FtpStreamChunk> + Send + 'static {
        // initialize output
        let ftp_inner_len = ftp.vector_completions.len();
        let mut output = Vec::with_capacity(ftp_inner_len);
        for _ in 0..ftp_inner_len {
            // safety: these should all be replaced without exception
            output.push(
                objectiveai::functions::expression::VectorCompletionOutput {
                    votes: Vec::new(),
                    scores: Vec::new(),
                    weights: Vec::new(),
                },
            );
        }

        // intiialize retry token
        let ftp_task_index_len = ftp.task_index_len();
        let mut retry_token = objectiveai::functions::executions::RetryToken(
            Vec::with_capacity(ftp_task_index_len),
        );
        for _ in 0..ftp_task_index_len {
            retry_token.0.push(None);
        }

        // combine all streams into one
        let stream = futures::stream::iter(
            ftp.vector_completions.into_iter().enumerate().map(
                move |(i, ftp)| {
                    futures::stream::once(
                        self.clone().execute_vector_ftp_streaming(
                            ctx.clone(),
                            request.clone(),
                            root_retry_token.clone(),
                            ftp,
                            task_index + i as u64,
                            choice_indexer.clone(),
                        ),
                    )
                    .flatten()
                },
            ),
        )
        .flatten();

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
                        retry_token: chunk_retry_token,
                    } => {
                        // get local index
                        let local_index =
                            (chunk_task_index - task_index) as usize;
                        // insert retry token into correct position
                        retry_token.insert(local_index, chunk_retry_token);
                        // insert output into correct position
                        output[local_index] = match chunk_output {
                            objectiveai::functions::expression::TaskOutputOwned::VectorCompletion(output) => output,
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
                output: objectiveai::functions::expression::TaskOutputOwned::MapVectorCompletion(output),
                retry_token,
            };
        }
    }

    async fn execute_vector_ftp_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
        root_retry_token: Option<
            Arc<objectiveai::functions::executions::RetryToken>,
        >,
        ftp: functions::VectorCompletionFlatTaskProfile,
        task_index: u64,
        choice_indexer: Arc<ChoiceIndexer>,
    ) -> impl Stream<Item = FtpStreamChunk> + Send + 'static {
        let request_base = request.base();
        let retry_token = root_retry_token
            .and_then(|rt| rt.0.get(task_index as usize).cloned())
            .flatten();
        let request_responses_len = ftp.responses.len();
        let mut stream = match self
            .vector_client
            .clone()
            .create_streaming_handle_usage(
                ctx,
                Arc::new(
                    objectiveai::vector::completions::request::VectorCompletionCreateParams {
                        retry: retry_token.clone(),
                        from_cache: request_base.from_cache,
                        from_rng: request_base.from_rng,
                        messages: ftp.messages,
                        provider: request_base.provider,
                        ensemble: objectiveai::vector::completions::request::Ensemble::Provided(
                            ftp.ensemble,
                        ),
                        profile: ftp.profile,
                        seed: request_base.seed,
                        stream: request_base.stream,
                        tools: ftp.tools,
                        backoff_max_elapsed_time: request_base
                            .backoff_max_elapsed_time,
                        first_chunk_timeout: request_base.first_chunk_timeout,
                        other_chunk_timeout: request_base.other_chunk_timeout,
                        responses: ftp.responses,
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
                            objectiveai::functions::executions::response::streaming::VectorCompletionTaskChunk {
                                index: choice_indexer.get(
                                    task_index as usize,
                                ),
                                task_index,
                                task_path: ftp.path.clone(),
                                inner: objectiveai::vector::completions::response::streaming::VectorCompletionChunk::default_from_request_responses_len(
                                    request_responses_len,
                                ),
                                error: Some(objectiveai::error::ResponseError::from(&e))
                            }
                        ),
                    ).chain(StreamOnce::new(
                        FtpStreamChunk::OutputChunk {
                            task_index,
                            output: objectiveai::functions::expression::TaskOutputOwned::VectorCompletion(
                                objectiveai::functions::expression::VectorCompletionOutput::default_from_request_responses_len(
                                    request_responses_len,
                                ),
                            ),
                            retry_token: objectiveai::functions::executions::RetryToken(vec![retry_token]),
                        }
                    )),
                );
            }
        };

        let mut aggregate: Option<
            objectiveai::vector::completions::response::streaming::VectorCompletionChunk,
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
                    objectiveai::functions::executions::response::streaming::VectorCompletionTaskChunk {
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
                retry_token: objectiveai::functions::executions::RetryToken(vec![{
                    let any_ok_completions = aggregate
                        .completions
                        .iter()
                        .any(|c| c.error.is_none());
                    if any_ok_completions {
                        Some(aggregate.id.clone())
                    } else {
                        // vector completion is not stored, so reuse same retry next time
                        // it is not stored because it succeeded 0 retries
                        retry_token
                    }
                }]),
                output: objectiveai::functions::expression::TaskOutputOwned::VectorCompletion(
                    objectiveai::functions::expression::VectorCompletionOutput::from(aggregate),
                ),
            };
        })
    }

    async fn create_reasoning_summary_streaming(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::executions::request::Request>,
        model: objectiveai::chat::completions::request::Model,
        models: Option<Vec<objectiveai::chat::completions::request::Model>>,
        description: Option<String>,
        output: objectiveai::functions::expression::FunctionOutput,
        confidence_responses: Vec<ConfidenceResponse>,
    ) -> impl Stream<Item = objectiveai::functions::executions::response::streaming::ReasoningSummaryChunk>
    + Send
    + 'static{
        // construct the prompt
        let mut parts = Vec::new();
        parts.push(objectiveai::chat::completions::request::RichContentPart::Text {
            text: match description {
                Some(description) => format!(
                    "The ObjectiveAI Function has the following description: \"{}\"\n\nThe user provided the following input to the ObjectiveAI Function:\n",
                    description,
                ),
                None => "The user provided the following input to an ObjectiveAI Function\n".to_string(),
            },
        });
        parts.extend(request.base().input.clone().to_rich_content_parts(0));
        parts.push(objectiveai::chat::completions::request::RichContentPart::Text {
            text: match output {
                objectiveai::functions::expression::FunctionOutput::Scalar(scalar) => {
                    format!(
                        "\n\nThe ObjectiveAI Function produced the following score: {}%\n\n",
                        (scalar * rust_decimal::dec!(100)).round_dp(2),
                    )
                },
                objectiveai::functions::expression::FunctionOutput::Vector(vector) => {
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
                objectiveai::functions::expression::FunctionOutput::Err(serde_json::Value::Number(n)) if {
                    n.as_f64().is_some()
                        && n.as_f64().unwrap() >= 0.0
                        && n.as_f64().unwrap() <= 1.0
                } => format!(
                    "\n\nThe ObjectiveAI Function erroneously produced the following score: {:.2}%\n\n",
                    n.as_f64().unwrap() * 100.0,
                ),
                objectiveai::functions::expression::FunctionOutput::Err(serde_json::Value::Array(arr)) if {
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
                objectiveai::functions::expression::FunctionOutput::Err(err) => format!(
                    "\n\nThe ObjectiveAI Function erroneously produced the following output:\n{}\n\n",
                    serde_json::to_string_pretty(&err).unwrap(),
                ),
            }
        });
        parts.push(objectiveai::chat::completions::request::RichContentPart::Text {
            text: "The ObjectiveAI Function used LLM Ensembles to arrive at this output by making assertions with associated confidence scores:\n\n".to_string(),
        });
        parts.extend(ConfidenceResponse::assertions(confidence_responses));
        parts.push(objectiveai::chat::completions::request::RichContentPart::Text {
            text: "\n\nYou are to present the output and summarize the reasoning process used by the ObjectiveAI Function to arrive at the output based on the assertions made above. Focus on the most confident assertions and explain how they contributed to the final output. If there were any low-confidence assertions, mention them with the caveat of low confidence. Provide a clear summary of the overall reasoning process.".to_string(),
        });

        // create the streaming chat completion
        let mut stream = match self
            .chat_client
            .clone()
            .create_streaming_for_chat_handle_usage(
                ctx,
                Arc::new(
                    objectiveai::chat::completions::request::ChatCompletionCreateParams {
                        messages: vec![objectiveai::chat::completions::request::Message::User(
                            objectiveai::chat::completions::request::UserMessage {
                                content:
                                    objectiveai::chat::completions::request::RichContent::Parts(
                                        parts,
                                    ),
                                name: None,
                            },
                        )],
                        provider: request.base().provider,
                        model,
                        models,
                        top_logprobs: None,
                        response_format: None,
                        seed: request.base().seed,
                        stream: Some(true),
                        tool_choice: None,
                        tools: None,
                        parallel_tool_calls: None,
                        prediction: None,
                        backoff_max_elapsed_time: request
                            .base()
                            .backoff_max_elapsed_time,
                        first_chunk_timeout: request.base().first_chunk_timeout,
                        other_chunk_timeout: request.base().other_chunk_timeout,
                    },
                ),
            )
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                return futures::future::Either::Left(StreamOnce::new(
                    objectiveai::functions::executions::response::streaming::ReasoningSummaryChunk {
                        inner: objectiveai::chat::completions::response::streaming::ChatCompletionChunk::default(),
                        error: Some(objectiveai::error::ResponseError::from(&e)),
                    }
                ));
            }
        };

        // only return error if the very first stream item is an error
        let mut next_chat_chunk = match stream.try_next().await {
            Ok(Some(chunk)) => Some(chunk),
            Err(e) => {
                return futures::future::Either::Left(StreamOnce::new(
                    objectiveai::functions::executions::response::streaming::ReasoningSummaryChunk {
                        inner: objectiveai::chat::completions::response::streaming::ChatCompletionChunk::default(),
                        error: Some(objectiveai::error::ResponseError::from(&e)),
                    }
                ));
            }
            Ok(None) => {
                // chat client will always yield at least one chunk
                unreachable!()
            }
        };

        // stream, buffered by 1 so as to attach errors
        futures::future::Either::Right(async_stream::stream! {
            while let Some(chat_chunk) = next_chat_chunk.take() {
                // fetch the next chat chunk or error
                let error = match stream.next().await {
                    Some(Ok(ncc)) => {
                        // set next chat chunk
                        next_chat_chunk = Some(ncc);
                        None
                    }
                    Some(Err(e)) => {
                        // end the loop after this iteration
                        // add error to choices
                        Some(objectiveai::error::ResponseError::from(&e))
                    }
                    None => {
                        // end the loop after this iteration
                        None
                    }
                };

                // yield the reasoning summary chunk
                yield objectiveai::functions::executions::response::streaming::ReasoningSummaryChunk {
                    inner: chat_chunk,
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
        objectiveai::functions::executions::response::streaming::VectorCompletionTaskChunk,
    ),
    /// A chunk from a nested Function execution.
    FunctionExecutionChunk(
        objectiveai::functions::executions::response::streaming::FunctionExecutionTaskChunk,
    ),
    /// The final output of a task with its retry token.
    OutputChunk {
        /// Index of the task in the flattened structure.
        task_index: u64,
        /// The computed output of the task.
        output: objectiveai::functions::expression::TaskOutputOwned,
        /// Token for retrying from this point.
        retry_token: objectiveai::functions::executions::RetryToken,
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
    pub response: objectiveai::chat::completions::request::RichContent,
    /// Aggregated confidence score.
    pub confidence: rust_decimal::Decimal,
    /// Collected reasoning from LLMs that voted for this response.
    pub reasoning: Vec<String>,
}

impl ConfidenceResponse {
    /// Formats all confidence responses as assertion parts for the reasoning prompt.
    pub fn assertions(
        confidence_responses: Vec<ConfidenceResponse>,
    ) -> impl Iterator<Item = objectiveai::chat::completions::request::RichContentPart>
    {
        confidence_responses
            .into_iter()
            .flat_map(ConfidenceResponse::assertion)
    }

    /// Formats this confidence response as JSON assertion parts.
    pub fn assertion(
        self,
    ) -> impl Iterator<Item = objectiveai::chat::completions::request::RichContentPart>
    {
        if self.confidence < rust_decimal::dec!(0.00005) {
            return None.into_iter().flatten();
        }
        Some(
            std::iter::once(objectiveai::chat::completions::request::RichContentPart::Text {
                text: "{\n    \"assertion\": \"".to_string(),
            })
            .chain({
                enum Iter<P> {
                    Text(Option<String>),
                    Parts(P),
                }
                impl<P: Iterator<Item = objectiveai::chat::completions::request::RichContentPart>>
                    Iterator for Iter<P>
                {
                    type Item = objectiveai::chat::completions::request::RichContentPart;
                    fn next(&mut self) -> Option<Self::Item> {
                        match self {
                        Iter::Text(opt_text) => {
                            opt_text.take().map(|text| {
                                objectiveai::chat::completions::request::RichContentPart::Text {
                                    text,
                                }
                            })
                        }
                        Iter::Parts(parts_iter) => parts_iter.next(),
                    }
                    }
                }
                match self.response {
                    objectiveai::chat::completions::request::RichContent::Text(text) => {
                        Iter::Text(Some(
                            json_escape::escape_str(&text).to_string(),
                        ))
                    }
                    objectiveai::chat::completions::request::RichContent::Parts(rich_parts) => {
                        Iter::Parts(rich_parts.into_iter().map(|part| {
                            if let objectiveai::chat::completions::request::RichContentPart::Text {
                            text,
                        } = part {
                            objectiveai::chat::completions::request::RichContentPart::Text {
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
                objectiveai::chat::completions::request::RichContentPart::Text {
                    text: format!(
                        "\",\n    \"confidence\": \"{}%\"",
                        (self.confidence * rust_decimal::dec!(100)).round_dp(2),
                    ),
                },
            ))
            .chain(std::iter::once(
                objectiveai::chat::completions::request::RichContentPart::Text {
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
