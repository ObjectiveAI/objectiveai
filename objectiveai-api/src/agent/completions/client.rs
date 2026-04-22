use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};

use crate::ctx;
use futures::StreamExt;

/// A function that transforms messages before they are sent to an upstream.
/// Keyed by agent ID so each agent in an swarm can receive different messages.
pub type TransformMessages = HashMap<
    String,
    Box<dyn Fn(Vec<objectiveai::agent::completions::message::Message>) -> Vec<objectiveai::agent::completions::message::Message> + Send + Sync>,
>;

pub fn response_id(created: u64) -> String {
    let uuid = uuid::Uuid::new_v4();
    format!("agtcpl-{}-{created}", uuid.simple())
}

// ---------------------------------------------------------------------------

/// Extracts the set of required MCP URLs from whichever continuation source is active.
/// Internal continuation takes precedence over request continuation.
fn required_mcp_urls<O, C, CC, M>(
    internal: Option<&super::Continuation<O, C, CC, M>>,
    request: Option<&objectiveai::agent::Continuation>,
) -> std::collections::HashSet<String> {
    if let Some(ic) = internal {
        ic.mcp_urls()
    } else if let Some(rc) = request {
        rc.mcp_sessions().keys().cloned().collect()
    } else {
        std::collections::HashSet::new()
    }
}

/// Filters agents to those whose MCP server URLs are a superset of `required_urls`
/// and (if `required_upstream` is set) match the required upstream type.
fn filter_agents(
    agents: Vec<objectiveai::agent::InlineAgent>,
    required_urls: &std::collections::HashSet<String>,
    required_upstream: Option<objectiveai::agent::Upstream>,
) -> Vec<objectiveai::agent::InlineAgent> {
    agents.into_iter().filter(|agent| {
        // Filter by upstream type if required.
        if let Some(upstream) = required_upstream {
            if agent.base().upstream() != upstream {
                return false;
            }
        }
        // Filter by MCP superset: agent's URLs must contain all required URLs.
        if required_urls.is_empty() {
            return true;
        }
        let agent_urls: std::collections::HashSet<&str> = agent.base().mcp_servers()
            .map(|s| s.iter().map(|s| s.url.as_str()).collect())
            .unwrap_or_default();
        required_urls.iter().all(|url| agent_urls.contains(url.as_str()))
    }).collect()
}

// ---------------------------------------------------------------------------

/// A shared, re-awaitable handle to a single MCP connection.
/// Uses `Arc<crate::mcp::Error>` so the result is `Clone` (required by `Shared`).
pub type McpHandle = futures::future::Shared<
    tokio::sync::oneshot::Receiver<
        Result<Arc<crate::mcp::Connection>, Arc<crate::mcp::Error>>
    >
>;

pub struct Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK, RETRG, RETRF, RETRM, CUSG> {
    /// MCP Client
    pub mcp_client: Arc<crate::mcp::Client>,
    /// Default MCP authorization headers (used when ctx doesn't provide them).
    pub mcp_authorization: Option<Arc<std::collections::HashMap<String, String>>>,
    /// Retrieve router for resolving remote agent references.
    pub retrieve_router: Arc<crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>>,
    /// Handler for tracking usage after completion.
    pub usage_handler: Arc<CUSG>,
    /// Upstream client for Openrouter agents.
    pub openrouter: Arc<OPENROUTER>,
    /// Upstream client for Claude Agent SDK agents.
    pub claude_agent_sdk: Arc<CLAUDEAGENTSDK>,
    /// Upstream client for Claude Code agents.
    pub claude_code: Arc<CLAUDECODE>,
    /// Upstream client for Mock agents.
    pub mock: Arc<MOCK>,
    /// Viewer client for streaming telemetry.
    pub viewer_client: Arc<crate::viewer::Client<CTXEXT>>,

    /// Current backoff interval for retry logic.
    pub backoff_current_interval: Duration,
    /// Initial backoff interval for retry logic.
    pub backoff_initial_interval: Duration,
    /// Randomization factor for backoff jitter.
    pub backoff_randomization_factor: f64,
    /// Multiplier for exponential backoff growth.
    pub backoff_multiplier: f64,
    /// Maximum backoff interval.
    pub backoff_max_interval: Duration,
    /// Maximum total time to spend on retries.
    pub backoff_max_elapsed_time: Duration,
    /// Maximum wait time for the first chunk in a streaming response.
    pub first_chunk_timeout: Duration,
    /// Maximum wait time between subsequent chunks in a streaming response.
    pub other_chunk_timeout: Duration,
    _marker: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK, RETRG, RETRF, RETRM, CUSG> Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK, RETRG, RETRF, RETRM, CUSG> {
    pub fn new(
        mcp_client: Arc<crate::mcp::Client>,
        mcp_authorization: Option<Arc<std::collections::HashMap<String, String>>>,
        retrieve_router: Arc<crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>>,
        usage_handler: Arc<CUSG>,
        openrouter: Arc<OPENROUTER>,
        claude_agent_sdk: Arc<CLAUDEAGENTSDK>,
        claude_code: Arc<CLAUDECODE>,
        mock: Arc<MOCK>,
        viewer_client: Arc<crate::viewer::Client<CTXEXT>>,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
        first_chunk_timeout: Duration,
        other_chunk_timeout: Duration,
    ) -> Self {
        Self {
            mcp_client,
            mcp_authorization,
            retrieve_router,
            usage_handler,
            openrouter,
            claude_agent_sdk,
            claude_code,
            mock,
            viewer_client,
            backoff_current_interval,
            backoff_initial_interval,
            backoff_randomization_factor,
            backoff_multiplier,
            backoff_max_interval,
            backoff_max_elapsed_time,
            first_chunk_timeout,
            other_chunk_timeout,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK, RETRG, RETRF, RETRM, CUSG> Clone
    for Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK, RETRG, RETRF, RETRM, CUSG>
{
    fn clone(&self) -> Self {
        Self {
            mcp_client: self.mcp_client.clone(),
            mcp_authorization: self.mcp_authorization.clone(),
            retrieve_router: self.retrieve_router.clone(),
            usage_handler: self.usage_handler.clone(),
            openrouter: self.openrouter.clone(),
            claude_agent_sdk: self.claude_agent_sdk.clone(),
            claude_code: self.claude_code.clone(),
            mock: self.mock.clone(),
            viewer_client: self.viewer_client.clone(),
            backoff_current_interval: self.backoff_current_interval,
            backoff_initial_interval: self.backoff_initial_interval,
            backoff_randomization_factor: self.backoff_randomization_factor,
            backoff_multiplier: self.backoff_multiplier,
            backoff_max_interval: self.backoff_max_interval,
            backoff_max_elapsed_time: self.backoff_max_elapsed_time,
            first_chunk_timeout: self.first_chunk_timeout,
            other_chunk_timeout: self.other_chunk_timeout,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK, RETRG, RETRF, RETRM, CUSG> Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK, RETRG, RETRF, RETRM, CUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: super::UpstreamClient<objectiveai::agent::openrouter::Agent, objectiveai::agent::openrouter::Continuation> + Send + Sync + 'static,
    CLAUDEAGENTSDK: super::UpstreamClient<objectiveai::agent::claude_agent_sdk::Agent, objectiveai::agent::claude_agent_sdk::Continuation> + Send + Sync + 'static,
    CLAUDECODE: super::UpstreamClient<objectiveai::agent::claude_code::Agent, objectiveai::agent::claude_code::Continuation> + Send + Sync + 'static,
    MOCK: super::UpstreamClient<objectiveai::agent::mock::Agent, objectiveai::agent::mock::Continuation> + Send + Sync + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT>,
    CUSG: super::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
{
    /// Creates a unary agent completion, tracking usage after completion.
    ///
    /// Internally streams the response and aggregates chunks into a single response.
    pub async fn create_unary_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        params: Arc<objectiveai::agent::completions::request::AgentCompletionCreateParams>,
        continuation: Option<
            super::Continuation<
                OPENROUTER::State,
                CLAUDEAGENTSDK::State,
                CLAUDECODE::State,
                MOCK::State,
            >,
        >,
        invention_tools: Option<Vec<objectiveai::functions::inventions::InventionTool>>,
        invention_done: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        transform_messages: Option<Arc<TransformMessages>>,
        viewer: bool,
        invention_type: Option<objectiveai::functions::inventions::prompts::StepPromptType>,
        invention_step: Option<usize>,
        invention_tasks_min: Option<u64>,
        invention_input_schema: Option<String>,
    ) -> Result<
        objectiveai::agent::completions::response::unary::AgentCompletion,
        super::Error,
    > {
        let mut aggregate: Option<
            objectiveai::agent::completions::response::streaming::AgentCompletionChunk,
        > = None;
        let mut stream = self
            .create_streaming_handle_usage(ctx, params, continuation, invention_tools, invention_done, transform_messages, viewer, invention_type, invention_step, invention_tasks_min, invention_input_schema)
            .await?;
        while let Some(item) = stream.next().await {
            match item {
                super::StreamItem::Chunk(chunk) => match &mut aggregate {
                    Some(agg) => agg.push(&chunk),
                    None => aggregate = Some(chunk),
                },
                super::StreamItem::State(_) => {}
            }
        }
        Ok(aggregate.unwrap().into())
    }

    /// Creates a streaming agent completion, tracking usage after the stream ends.
    pub async fn create_streaming_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        params: Arc<objectiveai::agent::completions::request::AgentCompletionCreateParams>,
        continuation: Option<
            super::Continuation<
                OPENROUTER::State,
                CLAUDEAGENTSDK::State,
                CLAUDECODE::State,
                MOCK::State,
            >,
        >,
        invention_tools: Option<Vec<objectiveai::functions::inventions::InventionTool>>,
        invention_done: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        transform_messages: Option<Arc<TransformMessages>>,
        viewer: bool,
        invention_type: Option<objectiveai::functions::inventions::prompts::StepPromptType>,
        invention_step: Option<usize>,
        invention_tasks_min: Option<u64>,
        invention_input_schema: Option<String>,
    ) -> Result<
        impl futures::Stream<
            Item = super::StreamItem<
                super::Continuation<
                    OPENROUTER::State,
                    CLAUDEAGENTSDK::State,
                    CLAUDECODE::State,
                    MOCK::State,
                >,
            >,
        > + Send
        + Unpin
        + 'static,
        super::Error,
    > {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = tokio::spawn(async move {
            let stream = match self
                .create_streaming(ctx.clone(), params.clone(), continuation, invention_tools, invention_done, transform_messages, viewer, invention_type, invention_step, invention_tasks_min, invention_input_schema)
                .await
            {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            let mut aggregate: Option<
                objectiveai::agent::completions::response::streaming::AgentCompletionChunk,
            > = None;
            futures::pin_mut!(stream);
            while let Some(item) = stream.next().await {
                match &item {
                    super::StreamItem::Chunk(chunk) => {
                        match &mut aggregate {
                            Some(agg) => agg.push(chunk),
                            None => aggregate = Some(chunk.clone()),
                        }
                    }
                    super::StreamItem::State(_) => {}
                }
                if tx.send(Ok(item)).is_err() {
                    ctx.cancel();
                }
            }
            drop(stream);
            drop(tx);
            let response: objectiveai::agent::completions::response::unary::AgentCompletion =
                aggregate.unwrap().into();
            if response.usage.any_usage() {
                self.usage_handler
                    .handle_usage(ctx, params, response)
                    .await;
            }
        });
        let mut stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        match stream.next().await {
            Some(Ok(first)) => Ok(
                futures::stream::iter(std::iter::once(first))
                    .chain(stream.map(Result::unwrap)),
            ),
            Some(Err(e)) => Err(e),
            None => unreachable!(),
        }
    }

    pub async fn create_streaming(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        params: Arc<objectiveai::agent::completions::request::AgentCompletionCreateParams>,
        continuation: Option<
            super::Continuation<
                OPENROUTER::State,
                CLAUDEAGENTSDK::State,
                CLAUDECODE::State,
                MOCK::State,
            >,
        >,
        invention_tools: Option<Vec<objectiveai::functions::inventions::InventionTool>>,
        invention_done: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        transform_messages: Option<Arc<TransformMessages>>,
        viewer: bool,
        invention_type: Option<objectiveai::functions::inventions::prompts::StepPromptType>,
        invention_step: Option<usize>,
        invention_tasks_min: Option<u64>,
        invention_input_schema: Option<String>,
    ) -> Result<
        impl futures::Stream<
            Item = super::StreamItem<
                super::Continuation<
                    OPENROUTER::State,
                    CLAUDEAGENTSDK::State,
                    CLAUDECODE::State,
                    MOCK::State,
                >,
            >,
        > + Send,
        super::Error,
    > {

        // Cancellation check closure factory — creates a new closure
        // that shares the same underlying AtomicBool via ctx's Arc.
        let make_is_cancelled = {
            let ctx = ctx.clone();
            move || {
                let ctx = ctx.clone();
                move || ctx.is_cancelled()
            }
        };

        // Parse request continuation from base64 string if provided.
        let request_continuation = match &params.continuation {
            Some(s) => Some(
                objectiveai::agent::Continuation::try_from_string(s)
                    .ok_or(super::Error::InvalidContinuation)?,
            ),
            None => None,
        };

        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let id = response_id(created);

        // Send viewer begin.
        if viewer {
            self.viewer_client.send_agent_completion_begin(
                ctx.clone(), id.clone(), params.clone(),
            );
        }
        let send_viewer_err = |e: super::Error| -> super::Error {
            if viewer {
                self.viewer_client.send_agent_completion_error(
                    ctx.clone(), id.clone(), &e,
                );
            }
            e
        };

        // 1. Panic if internal and request continuation upstream types conflict.
        if let (Some(ic), Some(rc)) = (&continuation, &request_continuation) {
            assert_eq!(
                ic.upstream(), rc.upstream(),
                "internal and request continuation upstream types must match"
            );
        }

        // 2. Extract continuation items, MCP connections, and upstream type.
        let cont_upstream = continuation.as_ref().map(|c| c.upstream());
        let (mut cont_items_or, mut cont_items_cas, mut cont_items_cc, mut cont_items_mock, internal_conns) = match continuation {
            Some(super::Continuation::Openrouter { items, mcp_connections }) => (items, vec![], vec![], vec![], Some(mcp_connections)),
            Some(super::Continuation::ClaudeAgentSdk { items, mcp_connections }) => (vec![], items, vec![], vec![], Some(mcp_connections)),
            Some(super::Continuation::ClaudeCode { items, mcp_connections }) => (vec![], vec![], items, vec![], Some(mcp_connections)),
            Some(super::Continuation::Mock { items, mcp_connections }) => (vec![], vec![], vec![], items, Some(mcp_connections)),
            None => (vec![], vec![], vec![], vec![], None),
        };

        // 3. Always resolve agents from params.agent.
        let agent_wf = self.retrieve_router.get_agent(&ctx, params.agent.clone()).await
            .map_err(|e| super::Error::InvalidAgent(e.message.to_string()))?;
        let inline = agent_wf.inline();
        let mut all_agents: Vec<objectiveai::agent::InlineAgent> = vec![inline.inner.clone()];
        if let Some(fallbacks) = &inline.fallbacks {
            all_agents.extend(fallbacks.iter().cloned());
        }

        // 4. Determine required MCP URLs and upstream type from continuation.
        let required_mcp_urls: std::collections::HashSet<String> = if let Some(conns) = &internal_conns {
            conns.iter().map(|c| c.url.clone()).collect()
        } else if let Some(rc) = &request_continuation {
            rc.mcp_sessions().keys().cloned().collect()
        } else {
            std::collections::HashSet::new()
        };
        let required_upstream = cont_upstream
            .or_else(|| request_continuation.as_ref().map(|c| c.upstream()));

        // 5. Filter agents by upstream type + MCP superset.
        let filtered_agents = filter_agents(all_agents, &required_mcp_urls, required_upstream);

        // 6. Spawn shared MCP connection map.
        let request_sessions = if internal_conns.is_none() {
            request_continuation.as_ref().map(|c| c.mcp_sessions())
        } else {
            None
        };
        let mcp_map = self.spawn_mcp_connection_map(
            &filtered_agents, &ctx, internal_conns.as_ref(), request_sessions,
        );

        // 7. Build agent attempts.
        struct AgentAttempt {
            agent: objectiveai::agent::InlineAgent,
            mcp_urls: Vec<String>,
        }
        let attempts: Vec<AgentAttempt> = filtered_agents.into_iter().map(|agent| {
            let mcp_urls = agent.base().mcp_servers()
                .map(|s| s.iter().map(|s| s.url.clone()).collect())
                .unwrap_or_default();
            AgentAttempt { agent, mcp_urls }
        }).collect();

        // 8. Backoff retry loop — try each agent in order.
        let mut backoff = backoff::ExponentialBackoff {
            current_interval: self.backoff_current_interval,
            initial_interval: self.backoff_initial_interval,
            randomization_factor: self.backoff_randomization_factor,
            multiplier: self.backoff_multiplier,
            max_interval: self.backoff_max_interval,
            start_time: std::time::Instant::now(),
            max_elapsed_time: Some(self.backoff_max_elapsed_time),
            clock: backoff::SystemClock::default(),
        };

        loop {
            let mut errors: Vec<super::Error> = Vec::new();

            for attempt in &attempts {
                // Await MCP connections for THIS agent from the shared map.
                let mut mcp_connections_vec = Vec::with_capacity(attempt.mcp_urls.len());
                let mut mcp_ok = true;
                for url in &attempt.mcp_urls {
                    let handle = mcp_map.get(url).expect("MCP URL missing from shared map");
                    match handle.clone().await.unwrap() {
                        Ok(conn) => mcp_connections_vec.push(conn),
                        Err(e) => {
                            errors.push(super::Error::McpConnectionArc(e));
                            mcp_ok = false;
                            break;
                        }
                    }
                }
                if !mcp_ok {
                    continue;
                }
                let mcp_connections = Arc::new(mcp_connections_vec);

                // a. List MCP tools for each connection.
                let mut mcp_tools = Vec::new();
                let mut mcp_ok = true;
                for conn in mcp_connections.iter() {
                    match conn.list_tools().await {
                        Ok(tools) => mcp_tools.push(tools),
                        Err(e) => {
                            errors.push(super::Error::McpListTools {
                                url: conn.url.clone(),
                                error: e,
                            });
                            mcp_ok = false;
                            break;
                        }
                    }
                }
                if !mcp_ok {
                    continue;
                }

                // b. Resolve response format for this agent.
                let response_format = resolve_response_format(attempt.agent.id(), &params);

                // c. Resolve tools.
                let (tool_names, tool_map) = super::tool::resolve_tools(
                    &mcp_connections,
                    &mcp_tools,
                    invention_tools.as_deref(),
                    response_format.as_ref(),
                );

                // d. Get BYOK for this agent's upstream.
                let byok = ctx.upstream_authorization(attempt.agent.base().upstream()).await;

                // e. BYOK strategy: try with key first, then without.
                let byok_attempts: Vec<Option<&str>> = match &byok {
                    Some(key) => vec![Some(key.as_str()), None],
                    None => vec![None],
                };

                let agent_transform = transform_messages.as_ref().and_then(|tm| {
                    tm.get(attempt.agent.id()).map(|f| f.as_ref())
                });

                for byok_attempt in &byok_attempts {
                    let err = match &attempt.agent {
                        objectiveai::agent::InlineAgent::Openrouter(or_agent) => {
                            let c = mcp_connections.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai::agent::Continuation::Openrouter(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.openrouter.clone(), or_agent, rc, &params, &mcp_connections,
                                invention_tools.as_deref(), &tool_names, &tool_map,
                                &mut cont_items_or, &id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                move |items| super::Continuation::Openrouter {
                                    items, mcp_connections: c,
                                },
                                |e| super::Error::UpstreamOpenrouter(Box::new(e)),
                                objectiveai::agent::InlineAgentRef::Openrouter(&or_agent.base),
                                invention_done.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                invention_type,
                                invention_step,
                                invention_tasks_min,
                                invention_input_schema.clone(),
                            ).await {
                                Ok(stream) => {
                                    if !viewer { return Ok(stream); }
                                    let vc = self.viewer_client.clone();
                                    let vctx = ctx.clone();
                                    return Ok(Box::pin(futures::StreamExt::inspect(stream, move |item| {
                                        if let super::StreamItem::Chunk(chunk) = item {
                                            vc.send_agent_completion_continue(vctx.clone(), chunk.clone());
                                        }
                                    })));
                                }
                                Err(e) => e,
                            }
                        }
                        objectiveai::agent::InlineAgent::ClaudeAgentSdk(cas_agent) => {
                            let c = mcp_connections.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai::agent::Continuation::ClaudeAgentSdk(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.claude_agent_sdk.clone(), cas_agent, rc, &params, &mcp_connections,
                                invention_tools.as_deref(), &tool_names, &tool_map,
                                &mut cont_items_cas, &id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                move |items| super::Continuation::ClaudeAgentSdk {
                                    items, mcp_connections: c,
                                },
                                |e| super::Error::UpstreamClaudeAgentSdk(Box::new(e)),
                                objectiveai::agent::InlineAgentRef::ClaudeAgentSdk(&cas_agent.base),
                                invention_done.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                invention_type,
                                invention_step,
                                invention_tasks_min,
                                invention_input_schema.clone(),
                            ).await {
                                Ok(stream) => {
                                    if !viewer { return Ok(stream); }
                                    let vc = self.viewer_client.clone();
                                    let vctx = ctx.clone();
                                    return Ok(Box::pin(futures::StreamExt::inspect(stream, move |item| {
                                        if let super::StreamItem::Chunk(chunk) = item {
                                            vc.send_agent_completion_continue(vctx.clone(), chunk.clone());
                                        }
                                    })));
                                }
                                Err(e) => e,
                            }
                        }
                        objectiveai::agent::InlineAgent::ClaudeCode(cc_agent) => {
                            let c = mcp_connections.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai::agent::Continuation::ClaudeCode(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.claude_code.clone(), cc_agent, rc, &params, &mcp_connections,
                                invention_tools.as_deref(), &tool_names, &tool_map,
                                &mut cont_items_cc, &id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                move |items| super::Continuation::ClaudeCode {
                                    items, mcp_connections: c,
                                },
                                |e| super::Error::UpstreamClaudeCode(Box::new(e)),
                                objectiveai::agent::InlineAgentRef::ClaudeCode(&cc_agent.base),
                                invention_done.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                invention_type,
                                invention_step,
                                invention_tasks_min,
                                invention_input_schema.clone(),
                            ).await {
                                Ok(stream) => {
                                    if !viewer { return Ok(stream); }
                                    let vc = self.viewer_client.clone();
                                    let vctx = ctx.clone();
                                    return Ok(Box::pin(futures::StreamExt::inspect(stream, move |item| {
                                        if let super::StreamItem::Chunk(chunk) = item {
                                            vc.send_agent_completion_continue(vctx.clone(), chunk.clone());
                                        }
                                    })));
                                }
                                Err(e) => e,
                            }
                        }
                        objectiveai::agent::InlineAgent::Mock(mock_agent) => {
                            let c = mcp_connections.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai::agent::Continuation::Mock(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.mock.clone(), mock_agent, rc, &params, &mcp_connections,
                                invention_tools.as_deref(), &tool_names, &tool_map,
                                &mut cont_items_mock, &id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                move |items| super::Continuation::Mock {
                                    items, mcp_connections: c,
                                },
                                |e| super::Error::UpstreamMock(Box::new(e)),
                                objectiveai::agent::InlineAgentRef::Mock(&mock_agent.base),
                                invention_done.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                invention_type,
                                invention_step,
                                invention_tasks_min,
                                invention_input_schema.clone(),
                            ).await {
                                Ok(stream) => {
                                    if !viewer { return Ok(stream); }
                                    let vc = self.viewer_client.clone();
                                    let vctx = ctx.clone();
                                    return Ok(Box::pin(futures::StreamExt::inspect(stream, move |item| {
                                        if let super::StreamItem::Chunk(chunk) = item {
                                            vc.send_agent_completion_continue(vctx.clone(), chunk.clone());
                                        }
                                    })));
                                }
                                Err(e) => e,
                            }
                        }
                    };
                    errors.push(err);
                }
            }

            // All agents failed this round — apply backoff or give up.
            if errors.is_empty() {
                return Err(send_viewer_err(super::Error::NoAgentsResolved));
            }
            use backoff::backoff::Backoff;
            match backoff.next_backoff() {
                Some(d) => tokio::time::sleep(d).await,
                None => {
                    return Err(send_viewer_err(if errors.len() == 1 {
                        errors.into_iter().next().unwrap()
                    } else {
                        super::Error::MultipleErrors(errors)
                    }));
                }
            }
        }
    }

    /// Creates an upstream stream and runs the tool-calling loop.
    ///
    /// 1. Calls `upstream.create()` with `first_chunk_timeout`.
    /// 2. Returns a stream that yields chunks as they arrive, executes
    ///    callable tools (MCP and invention), and re-invokes the upstream
    ///    for each continuation until no more callable tool calls remain.
    /// 3. The final stream item is always `StreamItem::State(CONT)`.
    ///
    /// On success, takes ownership of `cont_items` (via `std::mem::take`).
    /// On failure, `cont_items` remains intact for BYOK retry.
    async fn run_agent_loop<A, U, RC, CONT>(
        &self,
        upstream: Arc<U>,
        agent: &A,
        request_continuation: Option<&RC>,
        params: &objectiveai::agent::completions::request::AgentCompletionCreateParams,
        mcp_connections: &[Arc<crate::mcp::Connection>],
        invention_tools: Option<&[objectiveai::functions::inventions::InventionTool]>,
        tool_names: &[String],
        tool_map: &HashMap<String, super::tool::ResolvedTool>,
        cont_items: &mut Vec<super::ContinuationItem<U::State>>,
        id: &str,
        created: u64,
        byok: Option<&str>,
        cost_multiplier: rust_decimal::Decimal,
        wrap_continuation: impl FnOnce(Vec<super::ContinuationItem<U::State>>) -> CONT + Send + 'static,
        map_upstream_err: impl Fn(U::Error) -> super::Error + Send + 'static,
        agent_base: objectiveai::agent::InlineAgentRef<'_>,
        invention_done: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        transform_messages: Option<&(dyn Fn(Vec<objectiveai::agent::completions::message::Message>) -> Vec<objectiveai::agent::completions::message::Message> + Send + Sync)>,
        is_cancelled: impl Fn() -> bool + Send + Sync + 'static,
        invention_type: Option<objectiveai::functions::inventions::prompts::StepPromptType>,
        invention_step: Option<usize>,
        invention_tasks_min: Option<u64>,
        invention_input_schema: Option<String>,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = super::StreamItem<CONT>> + Send>>,
        super::Error,
    >
    where
        U: super::UpstreamClient<A, RC> + Send + Sync + 'static,
        A: Send + Sync + Clone + 'static,
        RC: Send + Sync + Clone + Into<objectiveai::agent::Continuation> + 'static,
        CONT: Send + 'static,
    {
        // --- Merge messages, prepare, and apply transform. ---
        let mut messages = agent_base.merged_messages(params.messages.clone());
        objectiveai::agent::completions::message::prompt::prepare(&mut messages);
        let messages = match transform_messages {
            Some(f) => f(messages),
            None => messages,
        };

        // --- Create the initial upstream stream with timeout. ---
        let cont_ref = if cont_items.is_empty() {
            None
        } else {
            Some(cont_items.as_slice())
        };
        let create_fut = upstream.create(
            id,
            created,
            agent,
            request_continuation.clone(),
            params,
            &messages,
            mcp_connections,
            invention_tools,
            tool_names,
            tool_map,
            cont_ref,
            byok,
            cost_multiplier,
            true,
            invention_type,
            invention_step,
            invention_tasks_min,
            invention_input_schema.clone(),
        );
        let initial_stream =
            tokio::time::timeout(self.first_chunk_timeout, create_fut)
                .await
                .map_err(|_| super::Error::Timeout)?
                .map_err(&map_upstream_err)?;

        // Success — take ownership of continuation items and build the stream.
        let mut continuation_items = std::mem::take(cont_items);
        let other_chunk_timeout = self.other_chunk_timeout;
        let agent = agent.clone();
        let mcp_connections = mcp_connections.to_vec();
        let params = params.clone();
        let invention_tools = invention_tools.map(|s| s.to_vec());
        let tool_names = tool_names.to_vec();
        let tool_map = tool_map.clone();
        let id = id.to_string();
        let byok = byok.map(|s| s.to_string());
        let request_continuation = request_continuation.cloned();

        Ok(Box::pin(async_stream::stream! {
            use objectiveai::agent::completions::message::{RichContent, ToolMessage};

            let mut aggregate: Option<
                objectiveai::agent::completions::response::streaming::AgentCompletionChunk,
            > = None;
            let mut usage =
                objectiveai::agent::completions::response::Usage::default();
            let mut upstream_kind = objectiveai::agent::Upstream::Unknown;
            let mut final_error: Option<objectiveai::error::ResponseError> = None;
            let mut stream: Pin<Box<dyn futures::Stream<Item = super::StreamItem<U::State>> + Send>> =
                Box::pin(initial_stream);
            loop {
                let mut current_state: Option<U::State> = None;
                let mut had_error = false;
                let mut pending_chunk: Option<
                    objectiveai::agent::completions::response::streaming::AgentCompletionChunk,
                > = None;

                loop {
                    match tokio::time::timeout(other_chunk_timeout, stream.next()).await {
                        Ok(Some(super::StreamItem::Chunk(chunk))) => {
                            // Import usage from assistant response chunks.
                            for msg in &chunk.messages {
                                if let objectiveai::agent::completions::response::streaming::MessageChunk::Assistant(asst) = msg {
                                    if let Some(upstream_usage) = &asst.usage {
                                        usage.push_upstream_usage(upstream_usage);
                                    }
                                }
                            }
                            // Track upstream from the first chunk that sets it.
                            if upstream_kind == objectiveai::agent::Upstream::Unknown
                                && chunk.upstream != objectiveai::agent::Upstream::Unknown
                            {
                                upstream_kind = chunk.upstream;
                            }
                            // An error chunk means the upstream failed mid-stream.
                            // Keep draining but prevent further continuation.
                            if chunk.error.is_some() {
                                had_error = true;
                            }
                            match &mut aggregate {
                                Some(agg) => agg.push(&chunk),
                                None => aggregate = Some(chunk.clone()),
                            }
                            // Yield the previous pending chunk (without usage),
                            // buffer the current one.
                            if let Some(prev) = pending_chunk.replace(chunk) {
                                yield super::StreamItem::Chunk(prev);
                            }
                        }
                        Ok(Some(super::StreamItem::State(state))) => {
                            current_state = Some(state);
                            break;
                        }
                        Ok(None) => break,
                        Err(_) => {
                            had_error = true;
                            break;
                        }
                    }
                }

                // Yield the last buffered chunk.
                if let Some(last) = pending_chunk.take() {
                    yield super::StreamItem::Chunk(last);
                }

                if had_error || is_cancelled() {
                    break;
                }

                let Some(ref agg) = aggregate else { break };

                let callable = extract_callable_tool_calls(agg, &tool_map);

                if callable.is_empty() {
                    break;
                }

                if let Some(state) = current_state.take() {
                    continuation_items.push(super::ContinuationItem::State(state));
                }

                let mut any_invention_tool_called = false;
                for (call_id, call_name, call_args) in &callable {
                    match tool_map.get(call_name) {
                        Some(super::tool::ResolvedTool::Mcp { connection, tool }) => {
                            let args: Option<indexmap::IndexMap<String, serde_json::Value>> =
                                serde_json::from_str(call_args).ok();
                            match connection
                                .call_tool_as_message(
                                    &crate::mcp::tool::CallToolRequestParams {
                                        name: tool.name.clone(),
                                        arguments: args,
                                        _meta: None,
                                        task: None,
                                    },
                                    call_id.clone(),
                                )
                                .await
                            {
                                Ok(tool_msg) => {
                                    let idx = continuation_items.len() as u64;
                                    let chunk = make_tool_chunk(&id, created, upstream_kind, idx, &tool_msg);
                                    if let Some(ref mut agg) = aggregate {
                                        agg.push(&chunk);
                                    }
                                    yield super::StreamItem::Chunk(chunk);
                                    continuation_items
                                        .push(super::ContinuationItem::ToolMessage(tool_msg));
                                }
                                Err(_) => {
                                    had_error = true;
                                    break;
                                }
                            }
                        }
                        Some(super::tool::ResolvedTool::InventionTool(inv)) => {
                            any_invention_tool_called = true;
                            let args: serde_json::Value = serde_json::from_str(call_args)
                                .unwrap_or(serde_json::Value::Object(Default::default()));
                            let content = match (inv.call)(args).await {
                                Ok(text) => text,
                                Err(text) => format!("Error: {text}"),
                            };
                            let tool_msg = ToolMessage {
                                content: RichContent::Text(content),
                                tool_call_id: call_id.clone(),
                            };
                            let idx = continuation_items.len() as u64;
                            let chunk = make_tool_chunk(&id, created, upstream_kind, idx, &tool_msg);
                            if let Some(ref mut agg) = aggregate {
                                agg.push(&chunk);
                            }
                            yield super::StreamItem::Chunk(chunk);
                            continuation_items
                                .push(super::ContinuationItem::ToolMessage(tool_msg));
                        }
                        _ => {}
                    }
                }

                if had_error {
                    break;
                }

                // When invention_done signals completion, disable tools so the
                // model responds with content and the loop terminates naturally.
                let tools_enabled = if any_invention_tool_called {
                    !invention_done.as_ref().is_some_and(|f| f())
                } else {
                    true
                };

                // Reset aggregate so the next iteration doesn't carry
                // old tool calls forward from the previous response.
                aggregate = None;

                match upstream
                    .create(
                        &id,
                        created,
                        &agent,
                        request_continuation.as_ref(),
                        &params,
                        &messages,
                        &mcp_connections,
                        invention_tools.as_deref(),
                        &tool_names,
                        &tool_map,
                        Some(&continuation_items),
                        byok.as_deref(),
                        cost_multiplier,
                        tools_enabled,
                        invention_type,
                        invention_step,
                        invention_tasks_min,
                        invention_input_schema.clone(),
                    )
                    .await
                {
                    Ok(new_stream) => {
                        stream = Box::pin(new_stream);
                    }
                    Err(e) => {
                        use objectiveai::error::StatusError;
                        let e = map_upstream_err(e);
                        final_error = Some(objectiveai::error::ResponseError {
                            code: e.status(),
                            message: e.message()
                                .unwrap_or(serde_json::Value::Null),
                        });
                        break;
                    }
                }
            }

            // Build MCP sessions map from active connections.
            let mcp_sessions: indexmap::IndexMap<String, String> = mcp_connections.iter()
                .map(|c| (c.url.clone(), c.session_id.clone()))
                .collect();

            // Build response continuation token.
            let response_cont = upstream.response_continuation(
                mcp_sessions,
                request_continuation.as_ref(),
                &messages,
                Some(&continuation_items),
            );
            let continuation_token: objectiveai::agent::Continuation = response_cont.into();
            let continuation_token = continuation_token.to_string();

            // Set cancellation error if the stream was cancelled.
            if is_cancelled() && final_error.is_none() {
                final_error = Some(objectiveai::error::ResponseError::from(
                    &super::Error::StreamCancelled,
                ));
            }

            // Single site for usage, continuation, and error (if a continuation call failed).
            yield super::StreamItem::Chunk(
                objectiveai::agent::completions::response::streaming::AgentCompletionChunk {
                    id: id.clone(),
                    created,
                    upstream: upstream_kind,
                    usage: Some(usage),
                    error: final_error,
                    continuation: Some(continuation_token),
                    ..Default::default()
                },
            );
            let cont = wrap_continuation(continuation_items);
            yield super::StreamItem::State(cont);
        }))
    }

    /// Spawns a shared map of MCP connections keyed by URL.
    ///
    /// Collects all unique MCP URLs across all agents, then for each URL:
    /// - If an internal continuation has a live connection for this URL, reuses it.
    /// - Otherwise, spawns a fresh connection (with session ID from request
    ///   continuation if available).
    ///
    /// Connections are shared: if two agents use the same MCP URL, they share
    /// one handle.
    pub fn spawn_mcp_connection_map(
        &self,
        agents: &[objectiveai::agent::InlineAgent],
        ctx: &crate::ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        internal_connections: Option<&Arc<Vec<Arc<crate::mcp::Connection>>>>,
        request_sessions: Option<&indexmap::IndexMap<String, String>>,
    ) -> HashMap<String, McpHandle> {
        // Collect all unique MCP URLs across all agents, with their authorization flag.
        let mut unique_urls: indexmap::IndexMap<String, bool> = indexmap::IndexMap::new();
        for agent in agents {
            if let Some(servers) = agent.base().mcp_servers() {
                for server in servers {
                    unique_urls.entry(server.url.clone())
                        .or_insert(server.authorization);
                }
            }
        }

        // Build a lookup of existing internal connections by URL.
        let internal_by_url: HashMap<&str, &Arc<crate::mcp::Connection>> = internal_connections
            .map(|conns| conns.iter().map(|c| (c.url.as_str(), c)).collect())
            .unwrap_or_default();

        let mut map = HashMap::with_capacity(unique_urls.len());

        for (url, requires_auth) in &unique_urls {
            // If internal continuation already has a live connection, reuse it.
            if let Some(existing) = internal_by_url.get(url.as_str()) {
                let conn = (*existing).clone();
                let (tx, rx) = tokio::sync::oneshot::channel();
                let _ = tx.send(Ok(conn));
                map.insert(url.clone(), futures::FutureExt::shared(rx));
                continue;
            }

            // Spawn a fresh connection.
            let map_key = url.clone();
            let url = url.clone();
            let requires_auth = *requires_auth;
            let session_id = request_sessions.and_then(|m| m.get(&url).cloned());
            let mcp_client = self.mcp_client.clone();
            let self_mcp_auth = self.mcp_authorization.clone();
            let ctx = ctx.clone();

            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::spawn(async move {
                let mcp_auth = ctx.mcp_authorization().await;
                let authorization = if requires_auth {
                    match mcp_auth.as_ref().and_then(|m| m.get(&url))
                        .or_else(|| self_mcp_auth.as_ref().and_then(|m| m.get(&url)))
                    {
                        Some(auth) => Some(auth.clone()),
                        None => {
                            let _ = tx.send(Err(Arc::new(crate::mcp::Error::MissingAuthorization(url))));
                            return;
                        }
                    }
                } else {
                    None
                };
                match mcp_client.connect(url, authorization, session_id).await {
                    Ok(conn) => { let _ = tx.send(Ok(conn)); }
                    Err(e) => { let _ = tx.send(Err(Arc::new(e))); }
                }
            });
            map.insert(map_key, futures::FutureExt::shared(rx));
        }

        map
    }
}

/// Resolves the response format for a given agent from the request params.
fn resolve_response_format(
    agent_id: &str,
    params: &objectiveai::agent::completions::request::AgentCompletionCreateParams,
) -> Option<objectiveai::agent::completions::request::ResponseFormat> {
    use objectiveai::agent::completions::request::ResponseFormatParam;
    match params.response_format.as_ref()? {
        ResponseFormatParam::Single(rf) => Some(rf.clone()),
        ResponseFormatParam::PerAgent(map) => map.get(agent_id).cloned(),
    }
}

/// Extracts callable tool calls (MCP and invention) from the accumulated chunk.
/// Returns `(call_id, resolved_tool_name, arguments_json)` for each callable tool.
fn extract_callable_tool_calls(
    aggregate: &objectiveai::agent::completions::response::streaming::AgentCompletionChunk,
    tool_map: &HashMap<String, super::tool::ResolvedTool>,
) -> Vec<(String, String, String)> {
    use objectiveai::agent::completions::response::streaming::MessageChunk;

    let mut callable = Vec::new();
    // Find the last assistant message and extract its accumulated tool calls.
    for msg in aggregate.messages.iter().rev() {
        if let MessageChunk::Assistant(chunk) = msg {
            if let Some(tool_calls) = &chunk.tool_calls {
                for tc in tool_calls {
                    let id = tc.id.clone().unwrap_or_default();
                    let name = tc
                        .function
                        .as_ref()
                        .and_then(|f| f.name.clone())
                        .unwrap_or_default();
                    let args = tc
                        .function
                        .as_ref()
                        .and_then(|f| f.arguments.clone())
                        .unwrap_or_default();
                    if name.is_empty() {
                        continue;
                    }
                    match tool_map.get(&name) {
                        Some(super::tool::ResolvedTool::Mcp { .. })
                        | Some(super::tool::ResolvedTool::InventionTool(_)) => {
                            callable.push((id, name, args));
                        }
                        _ => {}
                    }
                }
            }
            break; // only inspect the last assistant message
        }
    }
    callable
}

/// Builds an `AgentCompletionChunk` containing a single tool-response message.
fn make_tool_chunk(
    id: &str,
    created: u64,
    upstream: objectiveai::agent::Upstream,
    index: u64,
    tool_msg: &objectiveai::agent::completions::message::ToolMessage,
) -> objectiveai::agent::completions::response::streaming::AgentCompletionChunk {
    use objectiveai::agent::completions::response::streaming::{
        AgentCompletionChunk, MessageChunk,
    };
    use objectiveai::agent::completions::response::ToolResponse;
    AgentCompletionChunk {
        id: id.to_string(),
        created,
        upstream,
        messages: vec![MessageChunk::Tool(ToolResponse {
            role: Default::default(),
            index,
            inner: tool_msg.clone(),
        })],
        ..Default::default()
    }
}
