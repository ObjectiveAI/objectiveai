use std::sync::Arc;
use std::time;
use futures::{Stream, StreamExt};
use crate::ctx;
use crate::util::{ChoiceIndexer, StreamOnce};

use objectiveai_sdk::agent::completions::message::{Message, UserMessage, RichContent, RichContentPart};

type LaboratoryExecutionChunk =
    objectiveai_sdk::laboratories::executions::response::streaming::LaboratoryExecutionChunk;
type BuilderChunk =
    objectiveai_sdk::laboratories::executions::response::streaming::BuilderChunk;
type EvaluationChunk =
    objectiveai_sdk::laboratories::executions::response::streaming::EvaluationChunk;
type Object = objectiveai_sdk::laboratories::executions::response::streaming::Object;
type Params = objectiveai_sdk::laboratories::executions::request::LaboratoryExecutionCreateParams;

type Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK> =
    crate::agent::completions::Continuation<
        <OPENROUTER as crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
        >>::State,
        <CLAUDEAGENTSDK as crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        >>::State,
        <CODEXSDK as crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        >>::State,
        <MOCK as crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
        >>::State,
    >;

pub fn response_id(created: u64) -> String {
    crate::util::response_id(Some("lbexec"), created)
}

/// Laboratory client that runs builder agents in orchestrated environments
/// (Docker containers, GCP instances, etc.) with the embedded
/// objectiveai-mcp-filesystem binary.
pub struct Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, LUSG, ORCH> {
    pub agent_client: Arc<
        crate::agent::completions::Client<
            CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG,
        >,
    >,
    pub retrieve_router:
        Arc<crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>>,
    pub usage_handler: Arc<LUSG>,
    pub viewer: Arc<crate::viewer::Client<CTXEXT>>,
    pub orchestrator: Arc<ORCH>,
}

/// Add an MCP server address to an inline agent base.
fn inject_mcp_server(agent: &mut objectiveai_sdk::agent::InlineAgentBase, mcp_url: String) {
    let server = objectiveai_sdk::agent::McpServer {
        url: mcp_url,
        authorization: false,
    };
    match agent {
        objectiveai_sdk::agent::InlineAgentBase::Openrouter(b) => {
            b.mcp_servers.get_or_insert_with(Vec::new).push(server);
        }
        objectiveai_sdk::agent::InlineAgentBase::ClaudeAgentSdk(b) => {
            b.mcp_servers.get_or_insert_with(Vec::new).push(server);
        }
        objectiveai_sdk::agent::InlineAgentBase::CodexSdk(b) => {
            b.mcp_servers.get_or_insert_with(Vec::new).push(server);
        }
        objectiveai_sdk::agent::InlineAgentBase::Mock(b) => {
            b.mcp_servers.get_or_insert_with(Vec::new).push(server);
        }
    }
}

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, LUSG, ORCH>
    Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, LUSG, ORCH>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
        > + Send
        + Sync
        + 'static,
    CLAUDEAGENTSDK: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    CODEXSDK: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    MOCK: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
        > + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    CUSG: crate::agent::completions::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
    LUSG: crate::laboratories::executions::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
    ORCH: crate::laboratories::orchestrator::Orchestrator<CTXEXT>,
{
    pub async fn create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<Params>,
    ) -> Result<
        impl Stream<Item = LaboratoryExecutionChunk> + Send + 'static,
        super::Error,
    > {
        // Timestamp and identify the execution — before any awaits
        let created = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let id = response_id(created);
        let object = Object::LaboratoryExecutionChunk;

        // Send begin to viewer
        self.viewer.send_laboratory_execution_begin(
            ctx.clone(),
            id.clone(),
            request.clone(),
        );

        // Helper: send error to viewer and return it
        let send_err = |e: super::Error| -> super::Error {
            self.viewer.send_laboratory_execution_error(
                ctx.clone(),
                id.clone(),
                objectiveai_sdk::error::ResponseError::from(&e),
            );
            e
        };

        if request.builder_agents.is_empty() {
            return Err(send_err(super::Error::NoBuilderAgents));
        }

        if request.evaluation_agent.is_some() != request.evaluation_output_schema.is_some() {
            return Err(send_err(super::Error::EvaluationConfigMismatch));
        }

        // Spawn builder environments and resolve agents concurrently
        #[cfg(feature = "orchestrator-bollard")]
        let binaries: &[(&str, &[u8])] = &[(
            "objectiveai-mcp-filesystem",
            super::mcp_binary::MCP_FILESYSTEM_BINARY,
        )];
        #[cfg(not(feature = "orchestrator-bollard"))]
        let binaries: &[(&str, &[u8])] = &[];
        let orchestrator_fut = self.orchestrator.spawn_containers(
            &ctx,
            &request.docker_image,
            request.builder_agents.len(),
            &id,
            binaries,
            &[("PORT", "3000")],
        );
        let builder_resolve_futs: Vec<_> = request
            .builder_agents
            .iter()
            .map(|agent_ref| self.retrieve_router.get_agent(&ctx, agent_ref.clone()))
            .collect();
        let eval_resolve_fut = async {
            match &request.evaluation_agent {
                Some(eval_ref) => self.retrieve_router.get_agent(&ctx, eval_ref.clone())
                    .await
                    .map(Some)
                    .map_err(|e| super::Error::AgentCompletion(e.to_string())),
                None => Ok(None),
            }
        };
        let (mcp_urls, resolved_builder_agents, resolved_eval_agent) = tokio::try_join!(
            async { orchestrator_fut.await.map_err(|e| super::Error::Orchestrator(objectiveai_sdk::error::ResponseError::from(&e))) },
            async {
                futures::future::try_join_all(builder_resolve_futs)
                    .await
                    .map_err(|e| super::Error::AgentCompletion(e.to_string()))
            },
            eval_resolve_fut,
        )
        .map_err(&send_err)?;

        let eval_agent = resolved_eval_agent.map(|wf| {
            let eval_agent_base = wf.inline().inner.clone().into_base();
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
                objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                    inner: eval_agent_base,
                    fallbacks: None,
                },
            )
        });

        let mut builder_inline_agents = Vec::with_capacity(request.builder_agents.len());
        for (i, builder_agent_wf) in resolved_builder_agents.into_iter().enumerate() {
            let mut builder_agent_base = builder_agent_wf.inline().inner.clone().into_base();
            inject_mcp_server(&mut builder_agent_base, mcp_urls[i].clone());
            builder_inline_agents.push(builder_agent_base);
        }

        // Create agent completions for each builder concurrently
        let indexer = Arc::new(ChoiceIndexer::new(0));
        let agent_client = self.agent_client.clone();

        let streams: Vec<_> = builder_inline_agents
            .into_iter()
            .enumerate()
            .map(|(native_index, builder_agent_base)| {
                let agent_client = agent_client.clone();
                let ctx = ctx.clone();
                let request = request.clone();
                let indexer = indexer.clone();
                let id = id.clone();
                let agent_index = native_index as u64;

                let builder_agent_with_fallbacks = objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                    inner: builder_agent_base,
                    fallbacks: None,
                };
                let builder_agent =
                    objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
                        builder_agent_with_fallbacks,
                    );

                let params = Arc::new(
                    objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
                        messages: request.builder_messages.clone(),
                        provider: request.provider.clone(),
                        agent: builder_agent,
                        response_format: None,
                        seed: request.seed,
                        stream: Some(true),
                        continuation: request.builder_continuation.clone(),
                    },
                );

                Box::pin(async_stream::stream! {
                    let stream_result = agent_client
                        .create_streaming(ctx, params, None, None, vec![], indexmap::IndexMap::new(), None, false, None, None, None, None)
                        .await;

                    match stream_result {
                        Ok(stream) => {
                            futures::pin_mut!(stream);
                            while let Some(item) = stream.next().await {
                                match item {
                                    crate::agent::completions::StreamItem::Chunk(chunk) => {
                                        let completion_index = indexer.get(native_index);
                                        yield LaboratoryExecutionChunk {
                                            id: id.clone(),
                                            builders: vec![BuilderChunk {
                                                index: completion_index,
                                                agent_index,
                                                inner: chunk,
                                            }],
                                            evaluations: Vec::new(),
                                            error: None,
                                            created,
                                            object,
                                            usage: None,
                                        };
                                    }
                                    crate::agent::completions::StreamItem::State(_cont) => {
                                        // Continuation state — not used yet
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let completion_index = indexer.get(native_index);
                            yield LaboratoryExecutionChunk {
                                id: id.clone(),
                                builders: vec![BuilderChunk {
                                    index: completion_index,
                                    agent_index,
                                    inner: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                                        error: Some(objectiveai_sdk::error::ResponseError::from(&e)),
                                        ..Default::default()
                                    },
                                }],
                                evaluations: Vec::new(),
                                error: None,
                                created,
                                object,
                                usage: None,
                            };
                        }
                    }
                }) as std::pin::Pin<Box<dyn Stream<Item = LaboratoryExecutionChunk> + Send>>
            })
            .collect();

        let viewer_client = self.viewer.clone();
        let viewer_ctx = ctx.clone();
        let orchestrator = self.orchestrator.clone();
        let cleanup_ctx = ctx.clone();
        let this = self.clone();
        let mut merged = futures::stream::select_all(streams);
        Ok(async_stream::stream! {
            let mut accumulated_usage = objectiveai_sdk::agent::completions::response::Usage::default();
            let mut errored_agents: std::collections::HashSet<u64> = std::collections::HashSet::new();

            // Phase 1: drain all builder streams
            while let Some(chunk) = merged.next().await {
                for builder in &chunk.builders {
                    if let Some(u) = &builder.inner.usage {
                        accumulated_usage.push(u);
                    }
                    if builder.inner.error.is_some() {
                        errored_agents.insert(builder.agent_index);
                    }
                }
                viewer_client.send_laboratory_execution_continue(viewer_ctx.clone(), chunk.clone());
                yield chunk;
            }

            // Phase 2: spawn evaluations for non-errored builders (only if eval agent provided and not all builders errored)
            if let Some(ref eval_agent) = eval_agent && errored_agents.len() < request.builder_agents.len() {
                let num_agents = request.builder_agents.len() as u64;
                let eval_streams: Vec<_> = (0..num_agents)
                    .filter(|i| !errored_agents.contains(i))
                    .map(|agent_index| {
                        Box::pin(this.clone().create_evaluation_streaming(
                            ctx.clone(),
                            request.clone(),
                            id.clone(),
                            created,
                            object,
                            agent_index,
                            agent_index,
                            eval_agent.clone(),
                        )) as std::pin::Pin<Box<dyn Stream<Item = LaboratoryExecutionChunk> + Send>>
                    })
                    .collect();

                let mut eval_merged = futures::stream::select_all(eval_streams);
                while let Some(chunk) = eval_merged.next().await {
                    for evaluation in &chunk.evaluations {
                        if let Some(u) = &evaluation.inner.usage {
                            accumulated_usage.push(u);
                        }
                    }
                    viewer_client.send_laboratory_execution_continue(viewer_ctx.clone(), chunk.clone());
                    yield chunk;
                }
            }

            let final_chunk = LaboratoryExecutionChunk {
                id: id.clone(),
                builders: Vec::new(),
                evaluations: Vec::new(),
                error: None,
                created,
                object,
                usage: Some(accumulated_usage),
            };
            if !request.persist.unwrap_or(false) {
                let orch = orchestrator.clone();
                let cctx = cleanup_ctx.clone();
                let eid = id.clone();
                let num = request.builder_agents.len();
                tokio::spawn(async move { orch.cleanup_containers(&cctx, &eid, num).await });
            }
            viewer_client.send_laboratory_execution_continue(viewer_ctx.clone(), final_chunk.clone());
            yield final_chunk;
        })
    }

    /// Create a streaming evaluation for a single evaluation agent.
    ///
    /// Appends the evaluation schema to the messages, runs the agent completion,
    /// parses the response as `InputValue`, validates against the schema, and
    /// retries on error up to `max_evaluation_retries`.
    pub fn create_evaluation_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<Params>,
        id: String,
        created: u64,
        object: Object,
        evaluation_index: u64,
        agent_index: u64,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    ) -> impl Stream<Item = LaboratoryExecutionChunk> + Send + 'static {
        let agent_client = self.agent_client.clone();
        let max_retries = request.max_evaluation_retries.unwrap_or(3);

        // Build the schema prompt suffix
        let schema_text = format!(
            "## evaluation schema\n\n{}",
            serde_json::to_string_pretty(request.evaluation_output_schema.as_ref().unwrap()).unwrap(),
        );

        // Inject schema into messages: append to last user message or create one
        let mut messages = request.evaluation_messages.clone().unwrap();
        let mut injected = false;
        for msg in messages.iter_mut().rev() {
            if let Message::User(user) = msg {
                match &mut user.content {
                    RichContent::Text(t) => {
                        t.push_str("\n\n");
                        t.push_str(&schema_text);
                    }
                    RichContent::Parts(parts) => {
                        parts.push(RichContentPart::Text {
                            text: format!("\n\n{schema_text}"),
                        });
                    }
                }
                injected = true;
                break;
            }
        }
        if !injected {
            messages.push(Message::User(UserMessage {
                content: RichContent::Text(schema_text.clone()),
                name: None,
            }));
        }

        let params = Arc::new(
            objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
                messages,
                provider: request.provider.clone(),
                agent,
                response_format: None,
                seed: request.seed,
                stream: Some(true),
                continuation: request.evaluation_continuation.clone(),
            },
        );

        async_stream::stream! {
            let mut continuation: Option<Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>> = None;
            let mut retries = 0u32;

            loop {
                // Create agent completion stream
                let stream_result = agent_client
                    .create_streaming(
                        ctx.clone(),
                        params.clone(),
                        continuation.take(),
                        None, // disable_tools
                        vec![], // extra_mcp_servers
                        indexmap::IndexMap::new(), // extra_mcp_headers
                        None,
                        false,
                        None,
                        None,
                        None,
                        None,
                    )
                    .await;

                let mut accumulated_chunk: Option<objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk> = None;

                match stream_result {
                    Ok(stream) => {
                        futures::pin_mut!(stream);
                        while let Some(item) = stream.next().await {
                            match item {
                                crate::agent::completions::StreamItem::Chunk(chunk) => {
                                    match &mut accumulated_chunk {
                                        Some(acc) => acc.push(&chunk),
                                        None => accumulated_chunk = Some(chunk.clone()),
                                    }
                                    yield LaboratoryExecutionChunk {
                                        id: id.clone(),
                                        builders: Vec::new(),
                                        evaluations: vec![EvaluationChunk {
                                            index: evaluation_index,
                                            agent_index,
                                            inner: chunk,
                                            output: None,
                                        }],
                                        error: None,
                                        created,
                                        object,
                                        usage: None,
                                    };
                                }
                                crate::agent::completions::StreamItem::State(cont) => {
                                    continuation = Some(cont);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield LaboratoryExecutionChunk {
                            id: id.clone(),
                            builders: Vec::new(),
                            evaluations: vec![EvaluationChunk {
                                index: evaluation_index,
                                agent_index,
                                inner: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                                    error: Some(objectiveai_sdk::error::ResponseError::from(&e)),
                                    ..Default::default()
                                },
                                output: None,
                            }],
                            error: None,
                            created,
                            object,
                            usage: None,
                        };
                        break;
                    }
                }

                // Extract assistant content text from accumulated chunks
                let content_text = accumulated_chunk
                    .as_ref()
                    .and_then(|chunk| {
                        chunk.messages.iter().rev().find_map(|msg| {
                            if let objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(asst) = msg {
                                asst.content.as_ref().map(|c| match c {
                                    RichContent::Text(t) => t.clone(),
                                    RichContent::Parts(parts) => parts
                                        .iter()
                                        .filter_map(|p| match p {
                                            RichContentPart::Text { text } => Some(text.as_str()),
                                            _ => None,
                                        })
                                        .collect::<Vec<_>>()
                                        .join(""),
                                })
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or_default();

                // Parse as InputValue
                let parse_result: Result<objectiveai_sdk::functions::expression::InputValue, _> = {
                    let mut de = serde_json::Deserializer::from_str(&content_text);
                    serde_path_to_error::deserialize(&mut de)
                };

                match parse_result {
                    Ok(input_value) => {
                        // Validate against schema (if provided)
                        let valid = request
                            .evaluation_output_schema
                            .as_ref()
                            .unwrap()
                            .validate_input(&input_value);

                        if valid {
                            // Yield final chunk with output
                            yield LaboratoryExecutionChunk {
                                id: id.clone(),
                                builders: Vec::new(),
                                evaluations: vec![EvaluationChunk {
                                    index: evaluation_index,
                                    agent_index,
                                    inner: Default::default(),
                                    output: Some(input_value),
                                }],
                                error: None,
                                created,
                                object,
                                usage: None,
                            };
                            break;
                        }

                        // Schema validation failed
                        let err = super::Error::EvaluationOutputSchemaMismatch;
                        if retries >= max_retries {
                            yield LaboratoryExecutionChunk {
                                id: id.clone(),
                                builders: Vec::new(),
                                evaluations: vec![EvaluationChunk {
                                    index: evaluation_index,
                                    agent_index,
                                    inner: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                                        error: Some(objectiveai_sdk::error::ResponseError::from(&err)),
                                        ..Default::default()
                                    },
                                    output: None,
                                }],
                                error: None,
                                created,
                                object,
                                usage: None,
                            };
                            break;
                        }

                        // Retry with error message
                        let retry_msg = format!(
                            "{}\n\n## error\n\nevaluation output does not match schema",
                            schema_text,
                        );
                        if let Some(ref mut cont) = continuation {
                            cont.push_user_message(UserMessage {
                                content: RichContent::Text(retry_msg),
                                name: None,
                            });
                        }
                        retries += 1;
                    }
                    Err(parse_err) => {
                        // Parse failed
                        let err = super::Error::EvaluationOutputParse(parse_err.to_string());
                        if retries >= max_retries {
                            yield LaboratoryExecutionChunk {
                                id: id.clone(),
                                builders: Vec::new(),
                                evaluations: vec![EvaluationChunk {
                                    index: evaluation_index,
                                    agent_index,
                                    inner: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                                        error: Some(objectiveai_sdk::error::ResponseError::from(&err)),
                                        ..Default::default()
                                    },
                                    output: None,
                                }],
                                error: None,
                                created,
                                object,
                                usage: None,
                            };
                            break;
                        }

                        // Retry with parse error
                        let retry_msg = format!(
                            "{}\n\n## error\n\n{}",
                            schema_text, parse_err,
                        );
                        if let Some(ref mut cont) = continuation {
                            cont.push_user_message(UserMessage {
                                content: RichContent::Text(retry_msg),
                                name: None,
                            });
                        }
                        retries += 1;
                    }
                }
            }
        }
    }

    pub fn create_unary_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<Params>,
    ) -> impl std::future::Future<
        Output = Result<
            objectiveai_sdk::laboratories::executions::response::unary::LaboratoryExecution,
            super::Error,
        >,
    > + Send {
        async move {
            let mut aggregate: Option<LaboratoryExecutionChunk> = None;
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
    }

    pub fn create_streaming_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<Params>,
    ) -> impl std::future::Future<
        Output = Result<
            impl Stream<Item = LaboratoryExecutionChunk> + Send + Unpin + 'static,
            super::Error,
        >,
    > + Send {
        async move {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            tokio::spawn(async move {
                let mut aggregate: Option<LaboratoryExecutionChunk> = None;
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
                let response: objectiveai_sdk::laboratories::executions::response::unary::LaboratoryExecution =
                    aggregate.unwrap().into();
                if response.any_usage() {
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
}
