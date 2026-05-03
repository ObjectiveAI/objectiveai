use crate::{ctx, util::ChoiceIndexer};
use objectiveai::error::StatusError;
use futures::{Stream, StreamExt};
use std::{
    pin::Pin,
    sync::Arc,
    time,
};

type FunctionInventionChunk =
    objectiveai::functions::inventions::response::streaming::FunctionInventionChunk;
type RecursiveChunk =
    objectiveai::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk;
type RecursiveInventionChunk =
    objectiveai::functions::inventions::recursive::response::streaming::FunctionInventionChunk;
type RecursiveObject =
    objectiveai::functions::inventions::recursive::response::streaming::Object;

/// Generates a unique response ID for recursive Function inventions.
pub fn recursive_invention_response_id(created: u64) -> String {
    let uuid = uuid::Uuid::new_v4();
    format!("fninvr-{}-{}", uuid.simple(), created)
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for recursively inventing Functions.
///
/// Orchestrates the recursive invention flow: invents the root function,
/// then spawns child inventions for each placeholder task, recursing
/// based on depth. All child streams are merged concurrently — no waiting,
/// no collecting.
pub struct Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM, RIUSG> {
    pub invention_client: Arc<
        crate::functions::inventions::Client<
            CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM,
        >,
    >,
    pub viewer_client: Arc<crate::viewer::Client<CTXEXT>>,
    pub usage_handler: Arc<RIUSG>,
}

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM, RIUSG>
    Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM, RIUSG>
{
    pub fn new(
        invention_client: Arc<
            crate::functions::inventions::Client<
                CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM,
            >,
        >,
        viewer_client: Arc<crate::viewer::Client<CTXEXT>>,
        usage_handler: Arc<RIUSG>,
    ) -> Self {
        Self {
            invention_client,
            viewer_client,
            usage_handler,
        }
    }
}

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM, RIUSG>
    Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM, RIUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: crate::agent::completions::UpstreamClient<objectiveai::agent::openrouter::Agent, objectiveai::agent::openrouter::Continuation>
        + Send
        + Sync
        + 'static,
    CLAUDEAGENTSDK: crate::agent::completions::UpstreamClient<
            objectiveai::agent::claude_agent_sdk::Agent, objectiveai::agent::claude_agent_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    CODEXSDK: crate::agent::completions::UpstreamClient<
            objectiveai::agent::codex_sdk::Agent, objectiveai::agent::codex_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    MOCK: crate::agent::completions::UpstreamClient<objectiveai::agent::mock::Agent, objectiveai::agent::mock::Continuation>
        + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT>,
    CUSG: crate::agent::completions::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
    IUSG: crate::functions::inventions::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
    FFNG: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    FFNF: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    FFNM: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    RIUSG: super::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
{
    pub async fn create_unary_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams>,
    ) -> Result<
        objectiveai::functions::inventions::recursive::response::unary::FunctionInventionRecursive,
        super::Error,
    > {
        let mut aggregate: Option<RecursiveChunk> = None;
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

    pub async fn create_streaming_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams>,
    ) -> Result<
        impl Stream<Item = RecursiveChunk> + Send + Unpin + 'static,
        super::Error,
    > {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let self_clone = self.clone();
        tokio::spawn(async move {
            let mut aggregate: Option<RecursiveChunk> = None;
            let stream = match self_clone
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
            if let Some(aggregate) = aggregate {
                if aggregate.usage.as_ref().is_some_and(
                    objectiveai::agent::completions::response::Usage::any_usage,
                ) {
                    self_clone
                        .usage_handler
                        .handle_usage(ctx, request, aggregate.into())
                        .await;
                }
            }
        });
        let mut stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        match stream.next().await {
            Some(Ok(chunk)) => {
                Ok(crate::util::StreamOnce::new(chunk)
                    .chain(stream.map(Result::unwrap)))
            }
            Some(Err(e)) => Err(e),
            None => unreachable!(),
        }
    }

    pub async fn create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<objectiveai::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams>,
    ) -> Result<
        impl Stream<Item = RecursiveChunk> + Send + 'static,
        super::Error,
    > {
        // Resolve state (inline or from remote files).
        let resolved_state = self.invention_client.retrieve_router
            .get_function_invention_state(&ctx, request.state.clone())
            .await
            .map_err(|e| crate::functions::inventions::Error::InvalidState(e.to_string()))?
            .ok_or(crate::functions::inventions::Error::StateNotFound)?;

        let created = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let id = recursive_invention_response_id(created);

        // send begin to viewer
        self.viewer_client.send_function_invention_recursive_begin(
            ctx.clone(),
            id.clone(),
            request.clone(),
        );

        let is_scalar = match &resolved_state {
            objectiveai::functions::inventions::state::ParamsState::AlphaScalarBranch(_)
            | objectiveai::functions::inventions::state::ParamsState::AlphaScalarLeaf(_)
            | objectiveai::functions::inventions::state::ParamsState::AlphaScalar(_) => true,
            _ => false,
        };
        let object = if is_scalar {
            RecursiveObject::AlphaScalarFunctionInventionRecursiveChunk
        } else {
            RecursiveObject::AlphaVectorFunctionInventionRecursiveChunk
        };

        let choice_indexer = Arc::new(ChoiceIndexer::new(0));

        let viewer_client = self.viewer_client.clone();
        let viewer_ctx = ctx.clone();

        let inner = run_recursive(
            self.invention_client.clone(),
            ctx,
            request,
            resolved_state,
            id.clone(),
            created,
            object,
            choice_indexer,
            0, // native index for root
        );

        // Wrap the inner stream to:
        // 1. Accumulate usage from all invention chunks.
        // 2. Make inventions_errors sticky (once true, always true).
        // 3. Emit a terminal chunk with total usage and empty inventions.
        let stream: Pin<Box<dyn Stream<Item = RecursiveChunk> + Send>> =
            Box::pin(async_stream::stream! {
                let mut accumulated_usage =
                    objectiveai::agent::completions::response::Usage::default();
                let mut had_errors = false;
                futures::pin_mut!(inner);
                while let Some(mut chunk) = inner.next().await {
                    for inv in &chunk.inventions {
                        if let Some(u) = &inv.inner.usage {
                            accumulated_usage.push(u);
                        }
                    }
                    if chunk.inventions_errors == Some(true) {
                        had_errors = true;
                    }
                    if had_errors {
                        chunk.inventions_errors = Some(true);
                    }
                    yield chunk;
                }
                yield RecursiveChunk {
                    id,
                    inventions: vec![],
                    inventions_errors: if had_errors { Some(true) } else { None },
                    created,
                    object,
                    usage: Some(accumulated_usage),
                };
            });

        let stream = stream.inspect(move |chunk| {
            viewer_client.send_function_invention_recursive_continue(viewer_ctx.clone(), chunk.clone());
        });

        // Await the first chunk. If it contains an error, return Err.
        let mut stream = Box::pin(stream);
        match stream.next().await {
            Some(first) => {
                if first.inventions_errors == Some(true) {
                    // Extract the first error from the inventions
                    if let Some(err) = first.inventions.iter()
                        .find_map(|inv| inv.inner.error.clone())
                    {
                        return Err(super::Error::InventionFirstChunk(err));
                    }
                }
                Ok(crate::util::StreamOnce::new(first).chain(stream))
            }
            None => unreachable!(),
        }
    }
}

/// Recursively invents a function and all its placeholder children.
///
/// 1. Runs a single-level invention for the given state.
/// 2. Wraps each chunk with the assigned index and yields immediately.
/// 3. After the invention stream completes, extracts placeholder children
///    from the final state.
/// 4. Spawns a recursive invention for each child concurrently.
/// 5. Merges all child streams via `select_all` and yields their chunks.
/// 6. After all children complete, replaces placeholders with the invented
///    function paths.
fn run_recursive<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM>(
    invention_client: Arc<
        crate::functions::inventions::Client<
            CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM,
        >,
    >,
    ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    request: Arc<objectiveai::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams>,
    resolved_state: objectiveai::functions::inventions::ParamsState,
    id: String,
    created: u64,
    object: RecursiveObject,
    choice_indexer: Arc<ChoiceIndexer>,
    native_index: usize,
) -> Pin<Box<dyn Stream<Item = RecursiveChunk> + Send>>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: crate::agent::completions::UpstreamClient<objectiveai::agent::openrouter::Agent, objectiveai::agent::openrouter::Continuation>
        + Send
        + Sync
        + 'static,
    CLAUDEAGENTSDK: crate::agent::completions::UpstreamClient<
            objectiveai::agent::claude_agent_sdk::Agent, objectiveai::agent::claude_agent_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    CODEXSDK: crate::agent::completions::UpstreamClient<
            objectiveai::agent::codex_sdk::Agent, objectiveai::agent::codex_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    MOCK: crate::agent::completions::UpstreamClient<objectiveai::agent::mock::Agent, objectiveai::agent::mock::Continuation>
        + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT>,
    CUSG: crate::agent::completions::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
    IUSG: crate::functions::inventions::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
    FFNG: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    FFNF: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    FFNM: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
{
    Box::pin(async_stream::stream! {
        // Build the single-level invention request using the resolved state (Inline).
        let resolved_state_for_error = resolved_state.clone();
        let invention_request = Arc::new(
            objectiveai::functions::inventions::request::FunctionInventionCreateParams {
                remote: Some(request.remote),
                overwrite: request.overwrite,
                state: objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Inline(resolved_state),
                provider: request.provider.clone(),
                agent: request.agent.clone(),
                prompt: request.prompt.clone(),
                seed: request.seed,
                stream: request.stream,
                max_step_retries: request.max_step_retries,
                continuation: request.continuation.clone(),
            },
        );

        // Run the single-level invention.
        let stream = match invention_client
            .clone()
            .create_streaming_handle_usage(ctx.clone(), invention_request)
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                // Yield an error chunk with the resolved state included.
                yield RecursiveChunk {
                    id: id.clone(),
                    inventions: vec![RecursiveInventionChunk {
                        index: choice_indexer.get(native_index),
                        inner: FunctionInventionChunk {
                            id: id.clone(),
                            completions: vec![],
                            state: Some(resolved_state_for_error.route()),
                            path: None,
                            function: None,
                            created,
                            object: object.into(),
                            usage: None,
                            error: Some(objectiveai::error::ResponseError {
                                code: objectiveai::error::StatusError::status(&e),
                                message: objectiveai::error::StatusError::message(&e)
                                    .unwrap_or_else(|| serde_json::json!(e.to_string())),
                            }),
                        },
                    }],
                    inventions_errors: Some(true),
                    created,
                    object,
                    usage: None,
                };
                return;
            }
        };

        // Stream the single-level invention, wrapping each chunk.
        let mut final_state: Option<objectiveai::functions::inventions::State> = None;
        let mut final_path: Option<objectiveai::RemotePath> = None;
        let mut saved_function: Option<objectiveai::functions::FullRemoteFunction> = None;
        let mut had_error = false;

        futures::pin_mut!(stream);
        while let Some(mut chunk) = stream.next().await {
            if chunk.state.is_some() {
                final_state = chunk.state.clone();
            }
            if chunk.path.is_some() {
                final_path = chunk.path.clone();
            }
            if chunk.function.is_some() {
                saved_function = chunk.function.clone();
            }
            if chunk.error.is_some() {
                had_error = true;
            }
            // For branch states, strip the function from the chunk so we
            // only yield it once — after placeholder replacement.
            if final_state.as_ref().is_some_and(|s| !s.placeholder_children().is_empty()) {
                chunk.function = None;
            }
            yield RecursiveChunk {
                id: id.clone(),
                inventions: vec![RecursiveInventionChunk {
                    index: choice_indexer.get(native_index),
                    inner: chunk,
                }],
                inventions_errors: None,
                created,
                object,
                usage: None,
            };
        }
        drop(stream);

        // If the invention errored or produced no state, stop here.
        let mut state = match final_state {
            Some(state) if !had_error => state,
            _ => return,
        };

        // Extract placeholder children from the final state.
        let children = state.placeholder_children();
        if children.is_empty() {
            return;
        }

        // Spawn a recursive invention for each child concurrently.
        // Each child gets a unique native index for the choice indexer.
        let base_native = (native_index + 1) * 1000; // avoid collisions
        let mut child_streams: Vec<Pin<Box<dyn Stream<Item = RecursiveChunk> + Send>>> = Vec::new();

        for (i, child_state) in children.into_iter().enumerate() {
            let child_native = base_native + i;
            // Each sub-invention gets a deterministic seed that is
            // distinct from its siblings, derived from the parent
            // seed XOR `child_native`. Without this, every child
            // would inherit `request.seed` verbatim and the only
            // axis differentiating one sibling from another in the
            // mock RNG seed would be `tool_names` — which is read
            // from the agent's connection cache at the moment
            // `resolve_tools` is called and is therefore vulnerable
            // to listener-driven cache-refresh timing under load.
            // Mixing in `child_native` makes each sibling's RNG
            // input independent of cache snapshot timing, so a
            // load-induced perturbation that happens to flip one
            // sibling's `tool_names` snapshot does not propagate
            // into a different mock seed and a different output.
            let child_seed = request
                .seed
                .map(|s| s ^ (child_native as i64));

            // Build the child's recursive request with the child's state wrapped in Inline.
            let child_request = Arc::new(
                objectiveai::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams {
                    remote: request.remote,
                    overwrite: request.overwrite,
                    state: objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Inline(child_state.clone()),
                    provider: request.provider.clone(),
                    agent: request.agent.clone(),
                    prompt: request.prompt.clone(),
                    seed: child_seed,
                    stream: request.stream,
                    max_step_retries: request.max_step_retries,
                    continuation: request.continuation.clone(),
                },
            );

            child_streams.push(run_recursive(
                invention_client.clone(),
                ctx.clone(),
                child_request,
                child_state,
                id.clone(),
                created,
                object,
                choice_indexer.clone(),
                child_native,
            ));
        }

        // Merge all child streams, yield chunks immediately, and collect
        // child invention paths for placeholder replacement.
        let mut child_paths: Vec<objectiveai::RemotePath> = Vec::new();
        let mut merged = futures::stream::select_all(child_streams);
        while let Some(chunk) = merged.next().await {
            // Collect paths from child inventions as they complete.
            for invention in &chunk.inventions {
                if let Some(path) = &invention.inner.path {
                    child_paths.push(path.clone());
                }
            }
            yield chunk;
        }

        // All children are done. Replace placeholders on the root state
        // and re-publish the updated function. If anything fails, fall back
        // to the original (pre-replacement) function so exactly 1 function
        // is yielded per invention.
        if child_paths.is_empty() || final_path.is_none() {
            // No child paths or no original path — yield the saved function.
            yield RecursiveChunk {
                id: id.clone(),
                inventions: vec![RecursiveInventionChunk {
                    index: choice_indexer.get(native_index),
                    inner: FunctionInventionChunk {
                        id: id.clone(),
                        completions: vec![],
                        state: Some(state),
                        path: final_path,
                        function: saved_function,
                        created,
                        object: object.into(),
                        usage: None,
                        error: None,
                    },
                }],
                inventions_errors: None,
                created,
                object,
                usage: None,
            };
            return;
        }

        state.replace_placeholders(&child_paths);
        state.write_readme();
        let function = match state.build_function() {
            Some(f) => f,
            None => {
                // Build failed — yield the saved (pre-replacement) function.
                yield RecursiveChunk {
                    id: id.clone(),
                    inventions: vec![RecursiveInventionChunk {
                        index: choice_indexer.get(native_index),
                        inner: FunctionInventionChunk {
                            id: id.clone(),
                            completions: vec![],
                            state: Some(state),
                            path: final_path,
                            function: saved_function,
                            created,
                            object: object.into(),
                            usage: None,
                            error: None,
                        },
                    }],
                    inventions_errors: None,
                    created,
                    object,
                    usage: None,
                };
                return;
            }
        };

        let repo = state.name();
        let publish_files = state.serialize_into_files();
        let description = crate::functions::inventions::extract_description(&state);

        let (updated_path, publish_error) = match request.remote {
            objectiveai::Remote::Filesystem => {
                match crate::functions::inventions::publish_filesystem(
                    &invention_client.filesystem_client, &ctx, repo, &publish_files,
                ).await {
                    Ok(path) => (Some(path), None),
                    Err(e) => (None, Some(e)),
                }
            }
            objectiveai::Remote::Github => {
                match crate::functions::inventions::publish_github(
                    &invention_client.github_client,
                    &invention_client.filesystem_client,
                    &ctx, repo, &description,
                    &publish_files,
                ).await {
                    Ok(path) => (Some(path), None),
                    Err(e) => (None, Some(e)),
                }
            }
            objectiveai::Remote::Mock => (None, None),
        };

        // Yield the post-replacement function. On publish failure, fall back
        // to the saved function and original path.
        let (final_function, final_path, inventions_errors, error) = if let Some(publish_error) = publish_error {
            (
                saved_function,
                final_path,
                Some(true),
                Some(objectiveai::error::ResponseError {
                    code: publish_error.status(),
                    message: publish_error.message().unwrap_or(serde_json::Value::Null),
                }),
            )
        } else {
            (Some(function), updated_path, None, None)
        };
        yield RecursiveChunk {
            id: id.clone(),
            inventions: vec![RecursiveInventionChunk {
                index: choice_indexer.get(native_index),
                inner: FunctionInventionChunk {
                    id: id.clone(),
                    completions: vec![],
                    state: Some(state),
                    path: final_path,
                    function: final_function,
                    created,
                    object: object.into(),
                    usage: None,
                    error,
                },
            }],
            inventions_errors,
            created,
            object,
            usage: None,
        };
    })
}

