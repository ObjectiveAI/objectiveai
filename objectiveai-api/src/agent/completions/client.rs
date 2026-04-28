use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};

use crate::ctx;
use futures::StreamExt;
use indexmap::IndexMap;

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

/// Filters agents by upstream type (if required by the continuation) and
/// drops agents whose declared MCP servers can't be authorized — i.e. any
/// server with `requires_auth = true` for which we lack a value in
/// `request_mcp_auth` / `self.mcp_authorization`. The proxy connection
/// is per-agent now, so there's no "URL superset" filter anymore.
fn filter_agents(
    agents: Vec<objectiveai::agent::InlineAgent>,
    required_upstream: Option<objectiveai::agent::Upstream>,
    request_mcp_auth: Option<&std::collections::HashMap<String, String>>,
    default_mcp_auth: Option<&std::collections::HashMap<String, String>>,
) -> Vec<objectiveai::agent::InlineAgent> {
    agents
        .into_iter()
        .filter(|agent| {
            if let Some(upstream) = required_upstream {
                if agent.base().upstream() != upstream {
                    return false;
                }
            }
            if let Some(servers) = agent.base().mcp_servers() {
                for s in servers {
                    if s.authorization
                        && request_mcp_auth.and_then(|m| m.get(&s.url)).is_none()
                        && default_mcp_auth.and_then(|m| m.get(&s.url)).is_none()
                    {
                        return false;
                    }
                }
            }
            true
        })
        .collect()
}

// ---------------------------------------------------------------------------

pub struct Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, MOCK, RETRG, RETRF, RETRM, CUSG> {
    /// MCP Client
    pub mcp_client: Arc<objectiveai::mcp::Client>,
    /// Lazy in-process mcp-proxy used for every per-agent MCP connection.
    pub proxy_spawner: Arc<super::ProxySpawner>,
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

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, MOCK, RETRG, RETRF, RETRM, CUSG> Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, MOCK, RETRG, RETRF, RETRM, CUSG> {
    pub fn new(
        mcp_client: Arc<objectiveai::mcp::Client>,
        proxy_spawner: Arc<super::ProxySpawner>,
        mcp_authorization: Option<Arc<std::collections::HashMap<String, String>>>,
        retrieve_router: Arc<crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>>,
        usage_handler: Arc<CUSG>,
        openrouter: Arc<OPENROUTER>,
        claude_agent_sdk: Arc<CLAUDEAGENTSDK>,
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
            proxy_spawner,
            mcp_authorization,
            retrieve_router,
            usage_handler,
            openrouter,
            claude_agent_sdk,
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

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, MOCK, RETRG, RETRF, RETRM, CUSG> Clone
    for Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, MOCK, RETRG, RETRF, RETRM, CUSG>
{
    fn clone(&self) -> Self {
        Self {
            mcp_client: self.mcp_client.clone(),
            proxy_spawner: self.proxy_spawner.clone(),
            mcp_authorization: self.mcp_authorization.clone(),
            retrieve_router: self.retrieve_router.clone(),
            usage_handler: self.usage_handler.clone(),
            openrouter: self.openrouter.clone(),
            claude_agent_sdk: self.claude_agent_sdk.clone(),
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

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, MOCK, RETRG, RETRF, RETRM, CUSG> Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, MOCK, RETRG, RETRF, RETRM, CUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: super::UpstreamClient<objectiveai::agent::openrouter::Agent, objectiveai::agent::openrouter::Continuation> + Send + Sync + 'static,
    CLAUDEAGENTSDK: super::UpstreamClient<objectiveai::agent::claude_agent_sdk::Agent, objectiveai::agent::claude_agent_sdk::Continuation> + Send + Sync + 'static,
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

        // 2. Extract continuation items, MCP connection, and upstream type.
        let cont_upstream = continuation.as_ref().map(|c| c.upstream());
        let (
            mut cont_items_or,
            mut cont_items_cas,
            mut cont_items_mock,
            internal_conn,
        ) = match continuation {
            Some(super::Continuation::Openrouter { items, mcp_connection }) => {
                (items, vec![], vec![], mcp_connection)
            }
            Some(super::Continuation::ClaudeAgentSdk { items, mcp_connection }) => {
                (vec![], items, vec![], mcp_connection)
            }
            Some(super::Continuation::Mock { items, mcp_connection }) => {
                (vec![], vec![], items, mcp_connection)
            }
            None => (vec![], vec![], vec![], None),
        };

        // 3. Always resolve agents from params.agent.
        let agent_wf = self
            .retrieve_router
            .get_agent(&ctx, params.agent.clone())
            .await
            .map_err(|e| super::Error::InvalidAgent(e.message.to_string()))?;
        let inline = agent_wf.inline();
        let mut all_agents: Vec<objectiveai::agent::InlineAgent> = vec![inline.inner.clone()];
        if let Some(fallbacks) = &inline.fallbacks {
            all_agents.extend(fallbacks.iter().cloned());
        }

        // 4. Filter agents: drop those whose required upstream doesn't
        //    match the continuation, or whose required MCP authorization
        //    is missing.
        let required_upstream = cont_upstream
            .or_else(|| request_continuation.as_ref().map(|c| c.upstream()));
        let request_mcp_auth = ctx.mcp_authorization().await;
        let filtered_agents = filter_agents(
            all_agents,
            required_upstream,
            request_mcp_auth.as_deref(),
            self.mcp_authorization.as_deref(),
        );

        // 5. Spawn the invention server (if applicable) so its URL can
        //    be added to every per-agent proxy `X-MCP-Servers` list.
        //    Held on the stack for the rest of `create_streaming`'s
        //    lifetime; its `Drop` aborts the in-process server task.
        let invention_server = match invention_tools.as_ref() {
            Some(tools) if !tools.is_empty() => {
                Some(super::InventionServer::new(tools.clone()).await)
            }
            _ => None,
        };
        let invention_url = invention_server.as_ref().map(|s| s.url());

        // 6. Boot the in-process proxy (idempotent — first call wins,
        //    subsequent calls reuse the same handle) and kick off one
        //    connect per agent in parallel. Awaiting each `JoinHandle`
        //    inside the per-agent branch later means the round-trips
        //    overlap rather than serializing.
        let proxy_handle = self
            .proxy_spawner
            .get()
            .await
            .map_err(|e| send_viewer_err(super::Error::McpProxyBootstrap(e.to_string())))?;
        let proxy_url = proxy_handle.url.clone();

        let request_mcp_auth_owned = request_mcp_auth.clone();
        let default_mcp_auth_owned = self.mcp_authorization.clone();
        let internal_conn_for_resume = internal_conn.clone();

        let connect_handles: Vec<
            Option<
                tokio::task::JoinHandle<
                    Result<objectiveai::mcp::Connection, objectiveai::mcp::Error>,
                >,
            >,
        > = filtered_agents
            .iter()
            .map(|agent| {
                // Build the per-agent X-MCP-* header set.
                let mut urls: Vec<String> = agent
                    .base()
                    .mcp_servers()
                    .map(|s| s.iter().map(|s| s.url.clone()).collect())
                    .unwrap_or_default();
                if let Some(u) = &invention_url {
                    urls.push(u.clone());
                }

                // No MCP servers and no invention server → no proxy
                // connection needed for this agent. Skipping the spawn
                // also keeps the per-agent proxy session out of the
                // response continuation's `mcp_sessions` map for
                // requests that don't use MCP at all.
                if urls.is_empty() {
                    return None;
                }

                let mut auth_map: indexmap::IndexMap<String, String> =
                    indexmap::IndexMap::new();
                if let Some(servers) = agent.base().mcp_servers() {
                    for s in servers {
                        if let Some(v) = request_mcp_auth_owned
                            .as_deref()
                            .and_then(|m| m.get(&s.url))
                            .or_else(|| {
                                default_mcp_auth_owned
                                    .as_deref()
                                    .and_then(|m| m.get(&s.url))
                            })
                        {
                            auth_map.insert(s.url.clone(), v.clone());
                        }
                    }
                }

                // Forward the same innate identity headers
                // (User-Agent / X-Title / Referer / HTTP-Referer) the
                // mcp client stamps locally — the proxy re-emits them
                // on its outbound upstream calls so the upstreams see
                // the api server, not the proxy, as the originator.
                let mcp_inner_headers = self.mcp_client.headers();

                let extra_headers: indexmap::IndexMap<String, String> =
                    indexmap::indexmap! {
                        "X-MCP-Servers".to_string() => serde_json::to_string(&urls).unwrap(),
                        "X-MCP-Authorization".to_string() => serde_json::to_string(&auth_map).unwrap(),
                        "X-MCP-Headers".to_string() => serde_json::to_string(&mcp_inner_headers).unwrap(),
                    };

                let mcp_client = self.mcp_client.clone();
                let proxy_url = proxy_url.clone();
                // Resume the proxy session if we're continuing — the
                // upstream sessions already live behind it.
                let session_id = internal_conn_for_resume
                    .as_ref()
                    .map(|c| c.session_id.clone());
                Some(tokio::spawn(async move {
                    mcp_client
                        .connect(proxy_url, None, session_id, extra_headers)
                        .await
                }))
            })
            .collect();

        // 7. Build agent attempts. Each holds its own connect-handle
        //    JoinHandle (or None when the agent has no MCP work);
        //    the actual `await` happens inside the per-agent branch
        //    in step 8 below.
        struct AgentAttempt {
            agent: objectiveai::agent::InlineAgent,
            connect_handle: Option<
                tokio::task::JoinHandle<
                    Result<objectiveai::mcp::Connection, objectiveai::mcp::Error>,
                >,
            >,
        }
        let mut attempts: Vec<AgentAttempt> = filtered_agents
            .into_iter()
            .zip(connect_handles)
            .map(|(agent, connect_handle)| AgentAttempt { agent, connect_handle })
            .collect();
        // Slot of resolved-or-None per attempt — populated lazily on
        // first awaited iteration of the retry loop, reused across
        // backoff retries so we don't re-issue the connect.
        let mut attempt_connections: Vec<Option<objectiveai::mcp::Connection>> =
            (0..attempts.len()).map(|_| None).collect();
        let mut attempt_connect_done: Vec<bool> = (0..attempts.len()).map(|_| false).collect();

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

            for (idx, attempt) in attempts.iter_mut().enumerate() {
                // Resolve the per-agent proxy connect handle on first
                // visit. All N connects were spawned up-front so the
                // initialize round-trips overlap; awaiting individual
                // handles here is cheap on later retry iterations.
                if !attempt_connect_done[idx] {
                    attempt_connect_done[idx] = true;
                    if let Some(handle) = attempt.connect_handle.take() {
                        match handle.await.unwrap() {
                            Ok(conn) => attempt_connections[idx] = Some(conn),
                            Err(e) => errors.push(super::Error::McpConnection(e)),
                        }
                    }
                }
                // An agent whose declared MCP servers are empty has no
                // connection but is still allowed to run (no proxy
                // session needed); only skip when the agent declared
                // servers and the connect failed.
                let agent_needs_mcp = attempt.agent.base().mcp_servers().is_some()
                    || invention_url.is_some();
                let mcp_connection: Option<objectiveai::mcp::Connection> =
                    attempt_connections[idx].clone();
                if agent_needs_mcp && mcp_connection.is_none() {
                    continue;
                }

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
                            let c = mcp_connection.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai::agent::Continuation::Openrouter(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.openrouter.clone(), or_agent, rc, &params, mcp_connection.clone(),
                                &mut cont_items_or, &id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                move |items| super::Continuation::Openrouter {
                                    items, mcp_connection: c,
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
                            let c = mcp_connection.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai::agent::Continuation::ClaudeAgentSdk(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.claude_agent_sdk.clone(), cas_agent, rc, &params, mcp_connection.clone(),
                                &mut cont_items_cas, &id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                move |items| super::Continuation::ClaudeAgentSdk {
                                    items, mcp_connection: c,
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
                        objectiveai::agent::InlineAgent::Mock(mock_agent) => {
                            let c = mcp_connection.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai::agent::Continuation::Mock(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.mock.clone(), mock_agent, rc, &params, mcp_connection.clone(),
                                &mut cont_items_mock, &id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                move |items| super::Continuation::Mock {
                                    items, mcp_connection: c,
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
        mcp_connection: Option<objectiveai::mcp::Connection>,
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
            mcp_connection.clone(),
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

        // Resolve the proxy's tool name set once upfront. Used to
        // distinguish tool calls the orchestrator should dispatch
        // (proxy-routed MCP calls, including invention tools served
        // through the proxy) from tool calls the upstream encodes for
        // its own reasons (response_format, etc.).
        let mcp_tool_names: Option<std::collections::HashSet<String>> =
            if let Some(conn) = &mcp_connection {
                let tools = conn.list_tools().await.map_err(|e| super::Error::McpListTools {
                    url: conn.url.clone(),
                    error: e,
                })?;
                Some(tools.iter().map(|t| t.name.clone()).collect())
            } else {
                None
            };

        // Success — take ownership of continuation items and build the stream.
        let mut continuation_items = std::mem::take(cont_items);
        let other_chunk_timeout = self.other_chunk_timeout;
        let agent = agent.clone();
        let params = params.clone();
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
                let callable =
                    extract_callable_tool_calls(agg, mcp_tool_names.as_ref());
                if callable.is_empty() {
                    break;
                }

                if let Some(state) = current_state.take() {
                    continuation_items.push(super::ContinuationItem::State(state));
                }

                // Fan all tool calls in this turn out to the proxy in
                // parallel; the proxy itself multiplexes onto its
                // upstreams. Sequential `.await` per call would stack
                // RPC latencies. `join_all` keeps the order so we yield
                // tool messages in the same order the assistant called
                // them — the proxy's per-call latency dominates the
                // turn rather than the sum.
                let conn = mcp_connection
                    .as_ref()
                    .expect("callable extraction returns empty without a connection")
                    .clone();
                let dispatch_futs = callable
                    .iter()
                    .map(|(call_id, name, args)| {
                        let conn = conn.clone();
                        let call_id = call_id.clone();
                        let name = name.clone();
                        let args_json = args.clone();
                        async move {
                            let arguments: Option<
                                indexmap::IndexMap<String, serde_json::Value>,
                            > = serde_json::from_str(&args_json).ok();
                            conn.call_tool_as_message(
                                &objectiveai::mcp::tool::CallToolRequestParams {
                                    name,
                                    arguments,
                                    _meta: None,
                                    task: None,
                                },
                                call_id,
                            )
                            .await
                        }
                    })
                    .collect::<Vec<_>>();
                let results = futures::future::join_all(dispatch_futs).await;

                let mut any_invention_tool_called = false;
                for result in results {
                    match result {
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

                // `invention_done` is the sentinel the invention client
                // hands us to signal "the model produced enough invention
                // tasks; let it close out with a free-form response next
                // turn". We can't tell from here whether any of the tools
                // we just dispatched were invention tools (the proxy
                // hides that), so we let the sentinel decide on its own.
                let _ = &any_invention_tool_called;
                let tools_enabled = !invention_done.as_ref().is_some_and(|f| f());

                if had_error {
                    break;
                }

                // Reset aggregate so the next iteration doesn't carry
                // old tool calls forward from the previous response.
                aggregate = None;

                let _ = (&invention_type, &invention_step, &invention_tasks_min);
                match upstream
                    .create(
                        &id,
                        created,
                        &agent,
                        request_continuation.as_ref(),
                        &params,
                        &messages,
                        mcp_connection.clone(),
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
                            message: e.message().unwrap_or(serde_json::Value::Null),
                        });
                        break;
                    }
                }
            }

            // Build MCP sessions map. With the per-agent proxy connection,
            // this is at most one entry: `proxy_url → agent_session_id`.
            let mcp_sessions: IndexMap<String, String> = mcp_connection
                .as_ref()
                .map(|c| {
                    let mut m = IndexMap::new();
                    m.insert(c.url.clone(), c.session_id.clone());
                    m
                })
                .unwrap_or_default();

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

/// Extracts callable tool calls from the last assistant message.
///
/// `callable_names` is the set of tool names the proxy connection
/// advertises (or `None` when the agent has no MCP work to do — in
/// which case nothing is callable). Tool calls whose names aren't in
/// the set are response-format / hallucinated-tool calls that the
/// orchestrator should leave alone, so the loop terminates.
///
/// Returns `(call_id, tool_name, arguments_json)` for each callable
/// tool, in the order the assistant emitted them.
fn extract_callable_tool_calls(
    aggregate: &objectiveai::agent::completions::response::streaming::AgentCompletionChunk,
    callable_names: Option<&std::collections::HashSet<String>>,
) -> Vec<(String, String, String)> {
    use objectiveai::agent::completions::response::streaming::MessageChunk;

    let mut callable = Vec::new();
    let Some(names) = callable_names else { return callable };
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
                    if names.contains(&name) {
                        callable.push((id, name, args));
                    }
                }
            }
            break;
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
