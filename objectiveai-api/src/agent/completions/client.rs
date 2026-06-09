use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};

use crate::ctx;
use futures::StreamExt;
use indexmap::IndexMap;

/// A function that transforms messages before they are sent to an upstream.
/// Keyed by agent ID so each agent in an swarm can receive different messages.
pub type TransformMessages = HashMap<
    String,
    Box<dyn Fn(Vec<objectiveai_sdk::agent::completions::message::Message>) -> Vec<objectiveai_sdk::agent::completions::message::Message> + Send + Sync>,
>;

pub fn response_id(created: u64) -> String {
    crate::util::response_id(None, created)
}

// ---------------------------------------------------------------------------

/// Filters agents by upstream type (if required by the continuation) and
/// drops agents whose declared MCP servers can't be authorized — i.e. any
/// server with `requires_auth = true` for which we lack a value in
/// `request_mcp_auth` / `self.mcp_authorization`. The proxy connection
/// is per-agent now, so there's no "URL superset" filter anymore.
fn filter_agents(
    agents: Vec<objectiveai_sdk::agent::InlineAgent>,
    required_upstream: Option<objectiveai_sdk::agent::Upstream>,
    request_mcp_auth: Option<&std::collections::HashMap<String, String>>,
    default_mcp_auth: Option<&std::collections::HashMap<String, String>>,
) -> Vec<objectiveai_sdk::agent::InlineAgent> {
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

pub struct Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG> {
    /// MCP Client
    pub mcp_client: Arc<objectiveai_sdk::mcp::Client>,
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
    /// Upstream client for Codex SDK agents.
    pub codex_sdk: Arc<CODEXSDK>,
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

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG> Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG> {
    pub fn new(
        mcp_client: Arc<objectiveai_sdk::mcp::Client>,
        proxy_spawner: Arc<super::ProxySpawner>,
        mcp_authorization: Option<Arc<std::collections::HashMap<String, String>>>,
        retrieve_router: Arc<crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>>,
        usage_handler: Arc<CUSG>,
        openrouter: Arc<OPENROUTER>,
        claude_agent_sdk: Arc<CLAUDEAGENTSDK>,
        codex_sdk: Arc<CODEXSDK>,
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
            codex_sdk,
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

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG> Clone
    for Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG>
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
            codex_sdk: self.codex_sdk.clone(),
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

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG> Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: super::UpstreamClient<objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation> + Send + Sync + 'static,
    CLAUDEAGENTSDK: super::UpstreamClient<objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation> + Send + Sync + 'static,
    CODEXSDK: super::UpstreamClient<objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation> + Send + Sync + 'static,
    MOCK: super::UpstreamClient<objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation> + Send + Sync + 'static,
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
        params: Arc<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>,
        continuation: Option<
            super::Continuation<
                OPENROUTER::State,
                CLAUDEAGENTSDK::State,
                CODEXSDK::State,
                MOCK::State,
            >,
        >,
        disable_tools: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        extra_mcp_servers: Vec<super::ExtraMcpServer>,
        extra_mcp_headers: indexmap::IndexMap<String, String>,
        transform_messages: Option<Arc<TransformMessages>>,
        viewer: bool,
        invention_type: Option<objectiveai_sdk::functions::inventions::prompts::StepPromptType>,
        invention_step: Option<usize>,
        invention_tasks_min: Option<u64>,
        invention_input_schema: Option<String>,
    ) -> Result<
        objectiveai_sdk::agent::completions::response::unary::AgentCompletion,
        super::Error,
    > {
        let mut aggregate: Option<
            objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
        > = None;
        let mut stream = self
            .create_streaming_handle_usage(ctx, params, continuation, disable_tools, extra_mcp_servers, extra_mcp_headers, transform_messages, viewer, invention_type, invention_step, invention_tasks_min, invention_input_schema)
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
        params: Arc<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>,
        continuation: Option<
            super::Continuation<
                OPENROUTER::State,
                CLAUDEAGENTSDK::State,
                CODEXSDK::State,
                MOCK::State,
            >,
        >,
        disable_tools: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        extra_mcp_servers: Vec<super::ExtraMcpServer>,
        extra_mcp_headers: indexmap::IndexMap<String, String>,
        transform_messages: Option<Arc<TransformMessages>>,
        viewer: bool,
        invention_type: Option<objectiveai_sdk::functions::inventions::prompts::StepPromptType>,
        invention_step: Option<usize>,
        invention_tasks_min: Option<u64>,
        invention_input_schema: Option<String>,
    ) -> Result<
        impl futures::Stream<
            Item = super::StreamItem<
                super::Continuation<
                    OPENROUTER::State,
                    CLAUDEAGENTSDK::State,
                    CODEXSDK::State,
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
                .create_streaming(ctx.clone(), params.clone(), continuation, disable_tools, extra_mcp_servers, extra_mcp_headers, transform_messages, viewer, invention_type, invention_step, invention_tasks_min, invention_input_schema)
                .await
            {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            let mut aggregate: Option<
                objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
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
            let response: objectiveai_sdk::agent::completions::response::unary::AgentCompletion =
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
        params: Arc<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>,
        continuation: Option<
            super::Continuation<
                OPENROUTER::State,
                CLAUDEAGENTSDK::State,
                CODEXSDK::State,
                MOCK::State,
            >,
        >,
        disable_tools: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        // URLs to fold into every per-agent `X-MCP-Servers` header
        // *without* mutating the agent's own `mcp_servers` config — used
        // by the function-inventions orchestrator to plumb the shared
        // InventionServer URL through the proxy without affecting the
        // agent's content-derived ID.
        extra_mcp_servers: Vec<super::ExtraMcpServer>,
        // Headers to merge into the per-agent `X-MCP-Headers` map. The
        // proxy forwards these verbatim to every upstream it fans out
        // to. Used by the function-inventions orchestrator to send its
        // tenant id (`X-Invention-Session-Id`) to the shared
        // InventionServer.
        extra_mcp_headers: indexmap::IndexMap<String, String>,
        transform_messages: Option<Arc<TransformMessages>>,
        viewer: bool,
        invention_type: Option<objectiveai_sdk::functions::inventions::prompts::StepPromptType>,
        invention_step: Option<usize>,
        invention_tasks_min: Option<u64>,
        invention_input_schema: Option<String>,
    ) -> Result<
        impl futures::Stream<
            Item = super::StreamItem<
                super::Continuation<
                    OPENROUTER::State,
                    CLAUDEAGENTSDK::State,
                    CODEXSDK::State,
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
                objectiveai_sdk::agent::Continuation::try_from_string(s)
                    .ok_or(super::Error::InvalidContinuation)?,
            ),
            None => None,
        };

        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Capture the internal continuation's instance hierarchy before
        // `continuation` is consumed during item extraction below. The wire
        // composite (`request_continuation`) stays live until the per-slot
        // builder, where `continuation_agent_instance_hierarchy` is resolved.
        let continuation_internal_aih: Option<String> = continuation
            .as_ref()
            .map(|c| c.agent_instance_hierarchy().to_string());
        // Pre-yield errors correlate against an agent's response id, passed
        // explicitly at each call site (the primary's response id).
        let send_viewer_err = {
            let viewer_client = self.viewer_client.clone();
            let ctx_for_err = ctx.clone();
            move |response_id: &str, e: super::Error| -> super::Error {
                if viewer {
                    viewer_client.send_agent_completion_error(
                        ctx_for_err.clone(),
                        response_id.to_string(),
                        &e,
                    );
                }
                e
            }
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
            mut cont_items_cdx,
            mut cont_items_mock,
            internal_conn,
        ) = match continuation {
            Some(super::Continuation::Openrouter { items, mcp_connection, .. }) => {
                (items, vec![], vec![], vec![], mcp_connection)
            }
            Some(super::Continuation::ClaudeAgentSdk { items, mcp_connection, .. }) => {
                (vec![], items, vec![], vec![], mcp_connection)
            }
            Some(super::Continuation::CodexSdk { items, mcp_connection, .. }) => {
                (vec![], vec![], items, vec![], mcp_connection)
            }
            Some(super::Continuation::Mock { items, mcp_connection, .. }) => {
                (vec![], vec![], vec![], items, mcp_connection)
            }
            None => (vec![], vec![], vec![], vec![], None),
        };

        // 3. Always resolve agents from params.agent.
        // `agent_remote` is `Some(path)` when the WF was fetched from a
        // remote (stamped on every chunk + outbound MCP-proxy header),
        // `None` when the request supplied the WF inline.
        let (agent_wf, agent_remote) = self
            .retrieve_router
            .get_agent(&ctx, params.agent.clone())
            .await
            .map_err(|e| super::Error::InvalidAgent(e.message.to_string()))?;
        let inline = agent_wf.inline();
        // WF-level identity: concatenation of the primary id with all
        // fallback ids. See `InlineAgentWithFallbacks::full_id`. Same
        // value for every slot in this completion.
        let agent_full_id = inline.full_id();
        let mut all_agents: Vec<objectiveai_sdk::agent::InlineAgent> = vec![inline.inner.clone()];
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

        // Per-agent identities. `response_ids` are freshly minted, one per
        // slot, and stay pure (no dash) so the `-`-joined RESPONSE-IDS group
        // and X-OBJECTIVEAI-RESPONSE-ID are unaffected. An agent *instance* is
        // `{agent.id()}-{response_id}`; the hierarchy is the spawner lineage
        // (`ctx.agent_instance_hierarchy()`) joined by `/` with this instance.
        let response_ids: Vec<String> =
            filtered_agents.iter().map(|_| response_id(created)).collect();
        // Primary id for pre-yield viewer-error correlation (empty when no
        // agents survived filtering).
        let primary_response_id: String =
            response_ids.first().cloned().unwrap_or_default();
        // Reuse on resume: internal continuation first, else the wire
        // continuation; `None` on a fresh call.
        let continuation_agent_instance_hierarchy: Option<String> =
            continuation_internal_aih.or_else(|| {
                request_continuation
                    .as_ref()
                    .map(|c| c.agent_instance_hierarchy().to_string())
                    .filter(|s| !s.is_empty())
            });
        // When resuming, the hierarchy is fixed: every slot gets that exact
        // value. Otherwise build a fresh per-agent instance. The first
        // segment is the WF-level `agent_full_id` (constant across all
        // slots in this completion) followed by `-{response_id}` to
        // disambiguate slots.
        let agent_instance_hierarchies: Vec<String> =
            match &continuation_agent_instance_hierarchy {
                Some(h) => vec![h.clone(); filtered_agents.len()],
                None => filtered_agents
                    .iter()
                    .enumerate()
                    .map(|(i, _agent)| {
                        let agent_instance = format!("{}-{}", agent_full_id, response_ids[i]);
                        match ctx.agent_instance_hierarchy() {
                            Some(prefix) => format!("{prefix}/{agent_instance}"),
                            None => agent_instance,
                        }
                    })
                    .collect(),
            };

        // 5. Boot the in-process proxy (idempotent — first call wins,
        //    subsequent calls reuse the same handle) and kick off one
        //    connect per agent in parallel. Awaiting each `JoinHandle`
        //    inside the per-agent branch later means the round-trips
        //    overlap rather than serializing.
        let proxy_handle = self
            .proxy_spawner
            .get()
            .await
            .map_err(|e| send_viewer_err(&primary_response_id, super::Error::McpProxyBootstrap(e.to_string())))?;
        let proxy_url = proxy_handle.url.clone();

        let request_mcp_auth_owned = request_mcp_auth.clone();
        let default_mcp_auth_owned = self.mcp_authorization.clone();
        let internal_conn_for_resume = internal_conn.clone();
        // Client-side resume path: when the wire continuation carries
        // a `mcp_sessions[proxy_url]`, use it to resume the proxy
        // session so upstream MCP sessions (and therefore tool
        // subprocess `MCP_SESSION_ID`s) stay stable across separate
        // `agents message` / `agent completions create` invocations.
        // Without this fallback, every continuation turn creates a
        // fresh proxy session, which dials upstream MCPs fresh, and
        // any per-session tool state (counters, caches keyed on
        // `MCP_SESSION_ID`) resets per turn.
        let wire_proxy_session_id: Option<String> = request_continuation
            .as_ref()
            .and_then(|rc| rc.mcp_sessions().get(&proxy_url).cloned());

        // Per-agent reverse-attach registration. For every surviving
        // agent that declares `client_objectiveai_mcp`, register that
        // agent's `response_id` against the inbound WS's
        // `ReverseAttachHandle` — same value the proxy stamps on every
        // outbound reverse-channel request as
        // `X-OBJECTIVEAI-RESPONSE-ID`, so `route()` in
        // `objectiveai_mcp/routes.rs` finds the matching channel via a
        // header lookup against this exact id.
        //
        // Multiplicities preserved:
        // - **One response_id → many per-MCP routes from the proxy.**
        //   The proxy fans out one outbound HTTP MCP connection per
        //   upstream URL (`/objectiveai`, `/{owner}/{name}/{ver}/{mcp}`,
        //   ...). Every one carries the same `X-OBJECTIVEAI-RESPONSE-ID`;
        //   route() finds the channel once, the per-MCP discrimination
        //   happens downstream via the path-extracted `McpKind`.
        // - **One WS may serve many response_ids (swarm).** Each
        //   surviving agent registers its own response_id; all entries
        //   point to the same underlying `ReverseChannel`.
        //   `ReverseAttachGuard::drop` removes every registered id
        //   when the WS closes.
        // - **Continuations.** Each turn mints a fresh response_id and
        //   re-registers. Cross-turn upstream-session stability is
        //   handled by the proxy's own `Mcp-Session-Id` + the
        //   continuation's `mcp_sessions[proxy_url]` map (the proxy
        //   reuses its in-memory `Session` and overwrites cached
        //   transient headers with the new turn's response_id via
        //   `apply_transient_headers` before any reused-connection
        //   request goes out). The api-side registry key is
        //   deliberately a per-turn value because that's what
        //   `apply_transient_headers` keeps fresh.
        let agent_needs_reverse_attach: Vec<bool> = filtered_agents
            .iter()
            .enumerate()
            .map(|(i, agent)| {
                let needs = agent.base().client_objectiveai_mcp().is_some()
                    && ctx.mcp_port().is_some()
                    && ctx.reverse_attach().is_some();
                if needs {
                    // Both `mcp_port` and `reverse_attach` were just
                    // checked above; unwrap is safe.
                    ctx.reverse_attach()
                        .unwrap()
                        .register(response_ids[i].clone());
                }
                needs
            })
            .collect();
        let mcp_port_for_synth = ctx.mcp_port();

        // Dash-joined list of every per-agent response_id leaf in
        // this completion. Stamped identically on every per-agent
        // proxy connect (`X-OBJECTIVEAI-RESPONSE-IDS`) so cli-stream
        // learns the sibling set from whichever connect lands first
        // — driving the group-local loser sweep in
        // `ConduitMcpHandler::select_response_ids`.
        let response_ids_group: String = response_ids.join("-");

        let connect_handles: Vec<
            Option<
                tokio::task::JoinHandle<
                    Result<
                        (
                            objectiveai_sdk::mcp::Connection,
                            Option<
                                std::sync::Arc<Vec<objectiveai_sdk::mcp::tool::Tool>>,
                            >,
                        ),
                        std::sync::Arc<objectiveai_sdk::mcp::Error>,
                    >,
                >,
            >,
        > = filtered_agents
            .iter()
            .zip(agent_instance_hierarchies.iter())
            .zip(response_ids.iter())
            .zip(agent_needs_reverse_attach.iter().copied())
            .map(|(((agent, agent_instance_hierarchy), id), needs_reverse_attach)| {
                // Build the per-agent X-MCP-* header set: the agent's
                // declared `mcp_servers` plus any caller-supplied
                // `extra_mcp_servers` (e.g. the function-inventions
                // orchestrator's per-step InventionServer URL — kept
                // out of the agent's own config so its content-hashed
                // ID stays stable across runs).
                let mut urls: Vec<String> = agent
                    .base()
                    .mcp_servers()
                    .map(|s| s.iter().map(|s| s.url.clone()).collect())
                    .unwrap_or_default();
                urls.extend(extra_mcp_servers.iter().map(|s| s.url.clone()));

                // If the agent declares `client_objectiveai_mcp` AND
                // a WS-attached CLI is on the other end, emit one
                // synthetic URL per CLI-hosted MCP server. The proxy
                // dials each as an independent upstream; the API's
                // loopback MCP router parses the path back into a
                // [`McpKind`] and forwards over the WS conduit. The
                // CLI conduit treats each URL as a separate MCP
                // session with its own `Mcp-Session-Id`, no
                // aggregation, no tool renaming.
                //
                // - `/objectiveai` is emitted only when the agent
                //   actually needs the primary upstream (declared
                //   `tools`, set `objectiveai = true`, or declared an
                //   `executable` plugin — non-executable plugins are
                //   present purely for their `mcp_servers`).
                // - One `/{owner}/{name}/{version}/{mcp}` per declared
                //   plugin MCP server. Plugin args ride alongside as
                //   `X-OBJECTIVEAI-ARGUMENTS` (per-URL header), JSON-
                //   serialized in declaration order.
                let client_mcp_synthetic_urls: Vec<(
                    String,
                    Option<indexmap::IndexMap<String, Option<String>>>,
                )> = match (
                    needs_reverse_attach,
                    mcp_port_for_synth,
                    agent.base().client_objectiveai_mcp(),
                ) {
                    (true, Some(mcp_port), Some(client_mcp)) => {
                        let mut out: Vec<(
                            String,
                            Option<indexmap::IndexMap<String, Option<String>>>,
                        )> = Vec::new();
                        let needs_objectiveai = !client_mcp.tools.is_empty()
                            || client_mcp.objectiveai.unwrap_or(false)
                            || client_mcp.plugins.iter().any(|p| p.executable);
                        if needs_objectiveai {
                            out.push((
                                format!("http://127.0.0.1:{mcp_port}/objectiveai"),
                                None,
                            ));
                        }
                        for plugin in &client_mcp.plugins {
                            for entry in plugin.mcp_servers.as_deref().unwrap_or(&[]) {
                                let path = format!(
                                    "{owner}/{name}/{version}/{mcp}",
                                    owner = percent_encode_segment(&plugin.owner),
                                    name = percent_encode_segment(&plugin.name),
                                    version = percent_encode_segment(&plugin.version),
                                    mcp = percent_encode_segment(&entry.name),
                                );
                                out.push((
                                    format!("http://127.0.0.1:{mcp_port}/{path}"),
                                    entry.arguments.clone(),
                                ));
                            }
                        }
                        out
                    }
                    _ => Vec::new(),
                };
                urls.extend(client_mcp_synthetic_urls.iter().map(|(u, _)| u.clone()));

                // No MCP servers → no proxy
                // connection needed for this agent. Skipping the spawn
                // also keeps the per-agent proxy session out of the
                // response continuation's `mcp_sessions` map for
                // requests that don't use MCP at all.
                if urls.is_empty() {
                    return None;
                }

                // Build the per-URL header map sent as `X-MCP-Headers`
                // to the proxy. For each agent-declared server URL,
                // start from the orchestrator-supplied `extra_mcp_headers`
                // (e.g. the function-inventions tenant id) and layer on
                // any configured `Authorization` for that URL. For each
                // entry in `extra_mcp_servers`, start from
                // `extra_mcp_headers` and layer on its own per-server
                // headers (which win on conflict). The proxy stamps
                // each per-URL header set on every outbound request
                // to that upstream.
                let mut per_url_headers: indexmap::IndexMap<
                    String,
                    indexmap::IndexMap<String, String>,
                > = indexmap::IndexMap::new();
                if let Some(servers) = agent.base().mcp_servers() {
                    for s in servers {
                        let mut h = extra_mcp_headers.clone();
                        if let Some(v) = request_mcp_auth_owned
                            .as_deref()
                            .and_then(|m| m.get(&s.url))
                            .or_else(|| {
                                default_mcp_auth_owned
                                    .as_deref()
                                    .and_then(|m| m.get(&s.url))
                            })
                        {
                            h.insert("Authorization".to_string(), v.clone());
                        }
                        per_url_headers.insert(s.url.clone(), h);
                    }
                }
                for s in &extra_mcp_servers {
                    let entry = per_url_headers
                        .entry(s.url.clone())
                        .or_insert_with(|| extra_mcp_headers.clone());
                    if let Some(server_headers) = &s.headers {
                        for (k, v) in server_headers {
                            entry.insert(k.clone(), v.clone());
                        }
                    }
                }
                // Plugin URLs carry `X-OBJECTIVEAI-ARGUMENTS` —
                // JSON-serialized declaration-order IndexMap the CLI
                // uses to spawn `<plugin> mcp <mcp> begin --<k> [v]`.
                // The URL path carries the McpKind discriminator.
                // `X-OBJECTIVEAI-RESPONSE-ID` is NOT stamped per-URL —
                // it's session-global, transmitted at the top level
                // (`proxy_request_headers` below) and stored on the
                // proxy's `Session::transient_headers` for re-stamping
                // on every outbound request.
                for (url, args) in &client_mcp_synthetic_urls {
                    let entry = per_url_headers
                        .entry(url.clone())
                        .or_insert_with(|| extra_mcp_headers.clone());
                    if let Some(args) = args {
                        if let Ok(json) = serde_json::to_string(args) {
                            entry.insert(
                                "X-OBJECTIVEAI-ARGUMENTS".to_string(),
                                json,
                            );
                        }
                    }
                }

                // Stamp the three `X-OBJECTIVEAI-MCP-*` headers on
                // the `/objectiveai` per-URL entry. Stamped
                // unconditionally — the proxy ignores inbound
                // `X-MCP-Headers` entirely on its resume path (it
                // rebuilds the per-URL header bag from the AEAD-encoded
                // payload), so emitting them on a resume is inert.
                // `per_url_headers` only contains the `/objectiveai`
                // key when `client_mcp_synthetic_urls` synthesized it
                // above (driven by `needs_objectiveai`), so a missing
                // entry is the correct no-op signal.
                if let (Some(mcp_port), Some(client_mcp)) = (
                    mcp_port_for_synth,
                    agent.base().client_objectiveai_mcp(),
                ) {
                    let objectiveai_url =
                        format!("http://127.0.0.1:{mcp_port}/objectiveai");
                    if let Some(entry) =
                        per_url_headers.get_mut(&objectiveai_url)
                    {
                        for (k, v) in client_mcp.mcp_headers().to_headers() {
                            entry.insert(k, v);
                        }
                    }
                }

                // Both `agent_instance_hierarchy` and `id` here are the closure's
                // per-slot bindings (zipped in from `agent_instance_hierarchies` and
                // `response_ids` above). `agent_instance_hierarchy` is the full
                // hierarchy (caller-lineage joined with this slot's instance,
                // `{agent_full_id}-{response_id}`); `id` is this slot's pure
                // response id — same value used downstream as `attempt.id`
                // (chunk id, notify key) and `X-OBJECTIVEAI-RESPONSE-ID`.
                // `RESPONSE-IDS` is the dash-joined group of every
                // sibling response_id in this completion, stamped
                // identically on every per-agent connect.
                // `AGENT-ID` is the per-slot leaf id; `AGENT-FULL-ID`
                // is the WF-level id (same across slots);
                // `AGENT-REMOTE` is the JSON-encoded `RemotePath` when
                // the WF was fetched remotely and is **omitted entirely**
                // when the agent is inline — empty-string headers are
                // forbidden end-to-end (the proxy filters absent keys
                // out of `Session::transient_headers` naturally, and
                // the cli conduit's `require_transient` treats
                // AGENT-REMOTE as optional).
                let mut proxy_request_headers: indexmap::IndexMap<String, String> =
                    indexmap::indexmap! {
                        "X-MCP-Servers".to_string() => serde_json::to_string(&urls).unwrap(),
                        "X-MCP-Headers".to_string() => serde_json::to_string(&per_url_headers).unwrap(),
                        "X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY".to_string() => agent_instance_hierarchy.clone(),
                        "X-OBJECTIVEAI-AGENT-ID".to_string() => agent.id().to_string(),
                        "X-OBJECTIVEAI-AGENT-FULL-ID".to_string() => agent_full_id.clone(),
                        "X-OBJECTIVEAI-RESPONSE-ID".to_string() => id.clone(),
                        "X-OBJECTIVEAI-RESPONSE-IDS".to_string() => response_ids_group.clone(),
                    };
                if let Some(remote) = agent_remote.as_ref() {
                    if let Ok(serialized) = serde_json::to_string(remote) {
                        proxy_request_headers.insert(
                            "X-OBJECTIVEAI-AGENT-REMOTE".to_string(),
                            serialized,
                        );
                    }
                }

                let mcp_client = self.mcp_client.clone();
                let proxy_url = proxy_url.clone();
                // Resume the proxy session if we're continuing — the
                // upstream sessions already live behind it. Prefer the
                // internal continuation (server-side retry; in-memory
                // Connection still warm) over the wire continuation
                // (client-side resume; only the encoded session id is
                // available — proxy reconstructs upstreams via its
                // AEAD payload).
                let session_id = internal_conn_for_resume
                    .as_ref()
                    .map(|c| c.session_id.clone())
                    .or_else(|| wire_proxy_session_id.clone());
                // Only agents that declared `client_objectiveai_mcp`
                // need a `list_tools` round-trip — they're the ones
                // we have a declaration to validate against. Agents
                // with only `mcp_servers` / `extra_mcp_servers` skip
                // it and pay one fewer round-trip.
                let needs_list_tools = agent.base().client_objectiveai_mcp().is_some();
                // Per-agent spawn: connect → optionally list_tools.
                // Every agent's task runs concurrently with every
                // other agent's, so the proxy `initialize` round-trips
                // fan out in parallel. The list_tools result is the
                // union across every declared upstream; the CLI's
                // conduit applies its `X-OBJECTIVEAI-MCP-CONFIG`
                // filter to the synthetic-upstream slice.
                //
                // Plugin MCP upstreams are NOT dialed here — the CLI
                // dials them inside its `initialize` handler so each
                // upstream gets the proxy-supplied aggregate session
                // id for resume on continuation.
                //
                // Error type is `Arc<mcp::Error>` because the SDK's
                // `list_tools` returns shared-ref errors (the cached
                // refresh-tools task fills the same slot), so wrapping
                // the `connect` error in `Arc` matches the downstream
                // error handling shape uniformly.
                Some(tokio::spawn(async move {
                    let conn = mcp_client
                        .connect(proxy_url, session_id, Some(proxy_request_headers))
                        .await
                        .map_err(std::sync::Arc::new)?;
                    let tools = if needs_list_tools {
                        Some(conn.list_tools().await?)
                    } else {
                        None
                    };
                    Ok::<_, std::sync::Arc<objectiveai_sdk::mcp::Error>>((conn, tools))
                }))
            })
            .collect();

        // 7. Build agent attempts. Each holds its own connect-handle
        //    JoinHandle (or None when the agent has no MCP work);
        //    the actual `await` happens inside the per-agent branch
        //    in step 8 below.
        struct AgentAttempt {
            agent: objectiveai_sdk::agent::InlineAgent,
            connect_handle: Option<
                tokio::task::JoinHandle<
                    Result<
                        (
                            objectiveai_sdk::mcp::Connection,
                            Option<
                                std::sync::Arc<
                                    Vec<objectiveai_sdk::mcp::tool::Tool>,
                                >,
                            >,
                        ),
                        std::sync::Arc<objectiveai_sdk::mcp::Error>,
                    >,
                >,
            >,
            /// Composite per-slot agent id forwarded as
            /// `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` to the MCP proxy and (for
            /// runner-backed upstreams) as `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY` in
            /// the env dict the runner hands to its child SDK
            /// subprocess. Derived from the response id (see step
            /// 6.5 above).
            agent_instance_hierarchy: String,
            /// Per-slot response-id leaf — trailing slash-segment of
            /// `agent_instance_hierarchy`. The value passed into `run_agent_loop`
            /// as the `id` argument, which becomes
            /// `AgentCompletionChunk.id` and the value cli-stream's
            /// conduit cache + sibling-group sweep match on.
            id: String,
        }
        let mut attempts: Vec<AgentAttempt> = filtered_agents
            .into_iter()
            .zip(connect_handles)
            .zip(agent_instance_hierarchies)
            .zip(response_ids)
            .map(|(((agent, connect_handle), agent_instance_hierarchy), id)| AgentAttempt {
                agent,
                connect_handle,
                agent_instance_hierarchy,
                id,
            })
            .collect();
        // Slot of resolved-or-None per attempt — populated lazily on
        // first awaited iteration of the retry loop, reused across
        // backoff retries so we don't re-issue the connect.
        let mut attempt_connections: Vec<Option<objectiveai_sdk::mcp::Connection>> =
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
                // handles here is cheap on later retry iterations. The
                // spawned task also calls `list_tools()` on the new
                // connection — the union list returned across every
                // upstream — so we can immediately validate the
                // agent's declared
                // tools without an extra round-trip.
                if !attempt_connect_done[idx] {
                    attempt_connect_done[idx] = true;
                    if let Some(handle) = attempt.connect_handle.take() {
                        match handle.await.unwrap() {
                            Ok((conn, tools_opt)) => {
                                // Validate the agent's
                                // `client_objectiveai_mcp.tools`
                                // declaration against the actual
                                // upstream tool list. Missing any
                                // declared tool fails this attempt.
                                // `tools_opt` is `Some` iff the agent
                                // declared `client_objectiveai_mcp`
                                // (we only paid for the round-trip
                                // when there was a declaration to
                                // validate).
                                let declaration =
                                    attempt.agent.base().client_objectiveai_mcp();
                                let mut missing: Option<
                                    objectiveai_sdk::agent::ClientObjectiveaiMcpEntry,
                                > = None;
                                if let (Some(client_mcp), Some(tools)) =
                                    (declaration, tools_opt.as_ref())
                                {
                                    let returned: Vec<&str> = tools
                                        .iter()
                                        .map(|t| t.name.as_ref())
                                        .collect();
                                    for declared in &client_mcp.tools {
                                        let suffix = format!("_{}", declared.name);
                                        let present = returned.iter().any(|n| {
                                            *n == declared.name.as_str()
                                                || n.ends_with(&suffix)
                                        });
                                        if !present {
                                            missing = Some(declared.clone());
                                            break;
                                        }
                                    }
                                }
                                match missing {
                                    None => attempt_connections[idx] = Some(conn),
                                    Some(declared) => {
                                        errors.push(
                                            super::Error::ClientObjectiveaiMcpToolMissing {
                                                owner: declared.owner,
                                                name: declared.name,
                                                version: declared.version,
                                            },
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(super::Error::McpConnectionArc(e));
                            }
                        }
                    }
                }
                // An agent whose declared MCP servers are empty has no
                // connection but is still allowed to run (no proxy
                // session needed); only skip when the agent declared
                // servers and the connect failed.
                //
                // `client_objectiveai_mcp` declarations REQUIRE an
                // mcp_connection: on the SSE/unary path
                // (`ctx.reverse_attach()` is `None`) no synthetic URL
                // is added → `urls.is_empty()` → `connect_handle` is
                // `None` → `attempt_connections[idx]` is `None` →
                // we surface `ClientObjectiveaiMcpUnavailable` so the
                // caller knows reverse-attach was required but not
                // available.
                let agent_needs_mcp = attempt.agent.base().mcp_servers().is_some()
                    || !extra_mcp_servers.is_empty()
                    || attempt.agent.base().client_objectiveai_mcp().is_some();
                let mcp_connection: Option<objectiveai_sdk::mcp::Connection> =
                    attempt_connections[idx].clone();
                if agent_needs_mcp && mcp_connection.is_none() {
                    if attempt.agent.base().client_objectiveai_mcp().is_some()
                        && (ctx.mcp_port().is_none() || ctx.reverse_attach().is_none())
                    {
                        errors.push(super::Error::ClientObjectiveaiMcpUnavailable);
                    }
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
                        objectiveai_sdk::agent::InlineAgent::Openrouter(or_agent) => {
                            let c = mcp_connection.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai_sdk::agent::Continuation::Openrouter(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.openrouter.clone(), or_agent, rc, &params, mcp_connection.clone(),
                                ctx.reverse_attach().cloned(),
                                &mut cont_items_or, &attempt.id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                {
                                    let agent_instance_hierarchy = attempt.agent_instance_hierarchy.clone();
                                    move |items| super::Continuation::Openrouter {
                                        items, mcp_connection: c, agent_instance_hierarchy,
                                    }
                                },
                                |e| super::Error::UpstreamOpenrouter(Box::new(e)),
                                objectiveai_sdk::agent::InlineAgentRef::Openrouter(&or_agent.base),
                                disable_tools.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                invention_type,
                                invention_step,
                                invention_tasks_min,
                                invention_input_schema.clone(),
                                attempt.agent_instance_hierarchy.as_str(),
                                attempt.agent.id(),
                                agent_full_id.as_str(),
                                agent_remote.as_ref(),
                            ).await {
                                Ok(stream) => {
                                    if !viewer { return Ok(stream); }
                                    let vc = self.viewer_client.clone();
                                    let vctx = ctx.clone();
                                    let params_for_viewer = params.clone();
                                    let mut sent_begin = false;
                                    return Ok(Box::pin(futures::StreamExt::inspect(stream, move |item| {
                                        if let super::StreamItem::Chunk(chunk) = item {
                                            if !sent_begin {
                                                sent_begin = true;
                                                vc.send_agent_completion_begin(
                                                    vctx.clone(), chunk.id.clone(), params_for_viewer.clone(),
                                                );
                                            }
                                            vc.send_agent_completion_continue(vctx.clone(), chunk.clone());
                                        }
                                    })));
                                }
                                Err(e) => e,
                            }
                        }
                        objectiveai_sdk::agent::InlineAgent::ClaudeAgentSdk(cas_agent) => {
                            let c = mcp_connection.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai_sdk::agent::Continuation::ClaudeAgentSdk(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.claude_agent_sdk.clone(), cas_agent, rc, &params, mcp_connection.clone(),
                                ctx.reverse_attach().cloned(),
                                &mut cont_items_cas, &attempt.id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                {
                                    let agent_instance_hierarchy = attempt.agent_instance_hierarchy.clone();
                                    move |items| super::Continuation::ClaudeAgentSdk {
                                        items, mcp_connection: c, agent_instance_hierarchy,
                                    }
                                },
                                |e| super::Error::UpstreamClaudeAgentSdk(Box::new(e)),
                                objectiveai_sdk::agent::InlineAgentRef::ClaudeAgentSdk(&cas_agent.base),
                                disable_tools.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                invention_type,
                                invention_step,
                                invention_tasks_min,
                                invention_input_schema.clone(),
                                attempt.agent_instance_hierarchy.as_str(),
                                attempt.agent.id(),
                                agent_full_id.as_str(),
                                agent_remote.as_ref(),
                            ).await {
                                Ok(stream) => {
                                    if !viewer { return Ok(stream); }
                                    let vc = self.viewer_client.clone();
                                    let vctx = ctx.clone();
                                    let params_for_viewer = params.clone();
                                    let mut sent_begin = false;
                                    return Ok(Box::pin(futures::StreamExt::inspect(stream, move |item| {
                                        if let super::StreamItem::Chunk(chunk) = item {
                                            if !sent_begin {
                                                sent_begin = true;
                                                vc.send_agent_completion_begin(
                                                    vctx.clone(), chunk.id.clone(), params_for_viewer.clone(),
                                                );
                                            }
                                            vc.send_agent_completion_continue(vctx.clone(), chunk.clone());
                                        }
                                    })));
                                }
                                Err(e) => e,
                            }
                        }
                        objectiveai_sdk::agent::InlineAgent::CodexSdk(cdx_agent) => {
                            let c = mcp_connection.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai_sdk::agent::Continuation::CodexSdk(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.codex_sdk.clone(), cdx_agent, rc, &params, mcp_connection.clone(),
                                ctx.reverse_attach().cloned(),
                                &mut cont_items_cdx, &attempt.id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                {
                                    let agent_instance_hierarchy = attempt.agent_instance_hierarchy.clone();
                                    move |items| super::Continuation::CodexSdk {
                                        items, mcp_connection: c, agent_instance_hierarchy,
                                    }
                                },
                                |e| super::Error::UpstreamCodexSdk(Box::new(e)),
                                objectiveai_sdk::agent::InlineAgentRef::CodexSdk(&cdx_agent.base),
                                disable_tools.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                invention_type,
                                invention_step,
                                invention_tasks_min,
                                invention_input_schema.clone(),
                                attempt.agent_instance_hierarchy.as_str(),
                                attempt.agent.id(),
                                agent_full_id.as_str(),
                                agent_remote.as_ref(),
                            ).await {
                                Ok(stream) => {
                                    if !viewer { return Ok(stream); }
                                    let vc = self.viewer_client.clone();
                                    let vctx = ctx.clone();
                                    let params_for_viewer = params.clone();
                                    let mut sent_begin = false;
                                    return Ok(Box::pin(futures::StreamExt::inspect(stream, move |item| {
                                        if let super::StreamItem::Chunk(chunk) = item {
                                            if !sent_begin {
                                                sent_begin = true;
                                                vc.send_agent_completion_begin(
                                                    vctx.clone(), chunk.id.clone(), params_for_viewer.clone(),
                                                );
                                            }
                                            vc.send_agent_completion_continue(vctx.clone(), chunk.clone());
                                        }
                                    })));
                                }
                                Err(e) => e,
                            }
                        }
                        objectiveai_sdk::agent::InlineAgent::Mock(mock_agent) => {
                            let c = mcp_connection.clone();
                            let rc = match &request_continuation {
                                Some(objectiveai_sdk::agent::Continuation::Mock(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.mock.clone(), mock_agent, rc, &params, mcp_connection.clone(),
                                ctx.reverse_attach().cloned(),
                                &mut cont_items_mock, &attempt.id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                {
                                    let agent_instance_hierarchy = attempt.agent_instance_hierarchy.clone();
                                    move |items| super::Continuation::Mock {
                                        items, mcp_connection: c, agent_instance_hierarchy,
                                    }
                                },
                                |e| super::Error::UpstreamMock(Box::new(e)),
                                objectiveai_sdk::agent::InlineAgentRef::Mock(&mock_agent.base),
                                disable_tools.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                invention_type,
                                invention_step,
                                invention_tasks_min,
                                invention_input_schema.clone(),
                                attempt.agent_instance_hierarchy.as_str(),
                                attempt.agent.id(),
                                agent_full_id.as_str(),
                                agent_remote.as_ref(),
                            ).await {
                                Ok(stream) => {
                                    if !viewer { return Ok(stream); }
                                    let vc = self.viewer_client.clone();
                                    let vctx = ctx.clone();
                                    let params_for_viewer = params.clone();
                                    let mut sent_begin = false;
                                    return Ok(Box::pin(futures::StreamExt::inspect(stream, move |item| {
                                        if let super::StreamItem::Chunk(chunk) = item {
                                            if !sent_begin {
                                                sent_begin = true;
                                                vc.send_agent_completion_begin(
                                                    vctx.clone(), chunk.id.clone(), params_for_viewer.clone(),
                                                );
                                            }
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
                return Err(send_viewer_err(&primary_response_id, super::Error::NoAgentsResolved));
            }
            use backoff::backoff::Backoff;
            match backoff.next_backoff() {
                Some(d) => tokio::time::sleep(d).await,
                None => {
                    return Err(send_viewer_err(&primary_response_id, if errors.len() == 1 {
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
        params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
        mcp_connection: Option<objectiveai_sdk::mcp::Connection>,
        reverse_attach: Option<Arc<crate::objectiveai_mcp::ReverseAttachHandle>>,
        cont_items: &mut Vec<super::ContinuationItem<U::State>>,
        id: &str,
        created: u64,
        byok: Option<&str>,
        cost_multiplier: rust_decimal::Decimal,
        wrap_continuation: impl FnOnce(Vec<super::ContinuationItem<U::State>>) -> CONT + Send + 'static,
        map_upstream_err: impl Fn(U::Error) -> super::Error + Send + 'static,
        agent_base: objectiveai_sdk::agent::InlineAgentRef<'_>,
        disable_tools: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        transform_messages: Option<&(dyn Fn(Vec<objectiveai_sdk::agent::completions::message::Message>) -> Vec<objectiveai_sdk::agent::completions::message::Message> + Send + Sync)>,
        is_cancelled: impl Fn() -> bool + Send + Sync + 'static,
        invention_type: Option<objectiveai_sdk::functions::inventions::prompts::StepPromptType>,
        invention_step: Option<usize>,
        invention_tasks_min: Option<u64>,
        invention_input_schema: Option<String>,
        agent_instance_hierarchy_header: &str,
        agent_id: &str,
        agent_full_id: &str,
        agent_remote: Option<&objectiveai_sdk::RemotePath>,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = super::StreamItem<CONT>> + Send>>,
        super::Error,
    >
    where
        U: super::UpstreamClient<A, RC> + Send + Sync + 'static,
        A: Send + Sync + Clone + 'static,
        RC: Send + Sync + Clone + Into<objectiveai_sdk::agent::Continuation> + 'static,
        CONT: Send + 'static,
    {
        // --- Merge messages, drain proxy-queued notifications, prepare,
        // and apply transform. ---
        //
        // `merged_messages` injects the agent's personality (system_prompt,
        // prefix/suffix content, etc.) and bakes in `params.messages`. It
        // must run **only on a fresh conversation**. On resumption — either
        // a wire-level resume (`request_continuation.is_some()`) or an
        // in-process resume (`!cont_items.is_empty()`) — the merged prefix
        // is already part of the prior turn's accumulated state (e.g.
        // OpenRouter's `Continuation.messages`, Claude SDK's session). Re-
        // merging here would prepend a duplicate personality block on every
        // turn, snowballing linearly with the conversation length.
        //
        // The drained-notifications user message is inserted at the FRONT
        // of `messages` (index 0) so the notifications lead the prompt —
        // ahead of any system / developer / user content from the caller.
        // The prepare pass that follows still folds redundant consecutive
        // same-role messages, and `transform_messages` sees the drained
        // content like any other message. The proxy's tool-response path
        // still drains in-flight notifications during a turn; this
        // init-time drain covers the gap *between* turns — i.e. when the
        // previous turn ended without a tool call, or when the user is
        // starting a fresh continuation. On resumption with an empty merged
        // prefix the drain message simply leads the new turn's content,
        // landing before the continuation items in the upstream request.
        let resuming = request_continuation.is_some() || !cont_items.is_empty();
        let mut messages = if resuming {
            Vec::new()
        } else {
            agent_base.merged_messages(params.messages.clone())
        };

        // Drain the CLI's local prompt queue via the WS reverse-
        // attach. The queue lives in the CLI's postgres-backed
        // `message_queue`; the API asks for its current contents
        // via `read_message_queue`, joins every entry into a single
        // new user turn, and saves the entry ids. The ids are then
        // stamped onto the first assistant chunk yielded downstream
        // via `AssistantResponseChunk.request_message_ids` — a
        // signal to the consumer that those rows have been
        // consumed for this turn. The consumer owns row deletion
        // (no longer fires a separate `clear_message_queue` WS
        // call). If the upstream stream errors before any assistant
        // chunk lands, the ids drop on the floor and the queue
        // stays populated — the next turn re-reads it.
        //
        // The CLI now returns per-row data (no cross-row separator
        // splicing); we join here so the agent sees one User
        // message with `\n\n` separators between consumed rows.
        // The flat content-id list rides through to the first
        // assistant chunk via the stamp pass below.
        let mut queue_ids_to_clear: Vec<i64> = Vec::new();
        if let Some(handle) = &reverse_attach {
            let response = read_message_queue_via_ws(
                handle,
                agent_instance_hierarchy_header,
            )
            .await?;
            if !response.rows.is_empty() {
                use objectiveai_sdk::agent::completions::message::{
                    Message, RichContent, RichContentPart, UserMessage,
                };
                // Splice rows together with `"\n\n"` separators and
                // flatten the content_ids. Mirrors the joining the
                // CLI used to do server-side.
                let mut all_parts: Vec<RichContentPart> = Vec::new();
                for (i, row) in response.rows.into_iter().enumerate() {
                    if i > 0 {
                        all_parts.push(RichContentPart::Text {
                            text: "\n\n".to_string(),
                        });
                    }
                    queue_ids_to_clear.extend(row.content_ids);
                    match row.rich_content {
                        RichContent::Text(text) => {
                            all_parts.push(RichContentPart::Text { text });
                        }
                        RichContent::Parts(parts) => {
                            all_parts.extend(parts);
                        }
                    }
                }
                // Collapse single-text-part to RichContent::Text
                // (lossless) for the same wire shape as before.
                let rich_content = if all_parts.len() == 1
                    && matches!(all_parts.first(), Some(RichContentPart::Text { .. }))
                {
                    let Some(RichContentPart::Text { text }) =
                        all_parts.into_iter().next()
                    else {
                        unreachable!("matched single Text part above")
                    };
                    RichContent::Text(text)
                } else {
                    RichContent::Parts(all_parts)
                };
                // Insert AFTER the leading system/developer chain so
                // the agent sees its personality prefix first, then
                // the queued content arrives as one user turn, then
                // any caller-supplied content follows. On resumption
                // `messages` is empty so `insert_idx == 0` and the
                // queued message simply leads the new turn.
                let insert_idx = messages
                    .iter()
                    .position(|m| {
                        !matches!(m, Message::System(_) | Message::Developer(_))
                    })
                    .unwrap_or(messages.len());
                messages.insert(
                    insert_idx,
                    Message::User(UserMessage {
                        content: rich_content,
                        name: None,
                    }),
                );
            }
        }

        objectiveai_sdk::agent::completions::message::prompt::prepare(&mut messages);
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
            agent_instance_hierarchy_header,
            agent_id,
            agent_full_id,
            agent_remote,
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
        let agent_instance_hierarchy_header = agent_instance_hierarchy_header.to_string();
        let agent_id = agent_id.to_string();
        let agent_full_id = agent_full_id.to_string();
        let agent_remote = agent_remote.cloned();
        let request_continuation = request_continuation.cloned();

        Ok(Box::pin(async_stream::stream! {
            use objectiveai_sdk::agent::completions::message::{RichContent, ToolMessage};

            let mut aggregate: Option<
                objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
            > = None;
            let mut usage =
                objectiveai_sdk::agent::completions::response::Usage::default();
            let mut upstream_kind = objectiveai_sdk::agent::Upstream::Unknown;
            let mut final_error: Option<objectiveai_sdk::error::ResponseError> = None;
            let mut stream: Pin<Box<dyn futures::Stream<Item = super::StreamItem<U::State>> + Send>> =
                Box::pin(initial_stream);
            // In-band signal of queue consumption: stamp
            // `queue_ids_to_clear` onto the first
            // `MessageChunk::Assistant` we can find in an OK
            // outbound chunk. The downstream consumer owns row
            // deletion (no more `clear_message_queue` WS RPC).
            // Until the stamp lands, the ids ride in the local
            // `pending_request_message_ids` slot; if the upstream
            // errors first, they fall on the floor and the queue
            // stays populated for the next turn to re-read.
            let mut pending_request_message_ids: Option<Vec<i64>> =
                if queue_ids_to_clear.is_empty() {
                    None
                } else {
                    Some(std::mem::take(&mut queue_ids_to_clear))
                };
            loop {
                let mut current_state: Option<U::State> = None;
                let mut had_error = false;
                let mut pending_chunk: Option<
                    objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
                > = None;

                loop {
                    match tokio::time::timeout(other_chunk_timeout, stream.next()).await {
                        Ok(Some(super::StreamItem::Chunk(chunk))) => {
                            // Identity (`agent_instance_hierarchy`,
                            // `agent_id`, `agent_full_id`, `agent_remote`)
                            // is stamped at the upstream-client level
                            // when each chunk is constructed — no need
                            // to re-stamp here.
                            // Import usage from assistant response chunks.
                            for msg in &chunk.messages {
                                if let objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(asst) = msg {
                                    if let Some(upstream_usage) = &asst.usage {
                                        usage.push_upstream_usage(upstream_usage);
                                    }
                                }
                            }
                            // Track upstream from the first chunk that sets it.
                            if upstream_kind == objectiveai_sdk::agent::Upstream::Unknown
                                && chunk.upstream != objectiveai_sdk::agent::Upstream::Unknown
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
                            // buffer the current one. Before yielding, walk
                            // its messages and stamp `request_message_ids`
                            // onto the first assistant chunk we can find —
                            // single-shot per stream lifetime.
                            if let Some(mut prev) = pending_chunk.replace(chunk) {
                                if pending_request_message_ids.is_some() && prev.error.is_none() {
                                    for m in prev.messages.iter_mut() {
                                        if let objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(asst) = m {
                                            asst.request_message_ids = pending_request_message_ids.take();
                                            break;
                                        }
                                    }
                                }
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

                // Yield the last buffered chunk. Same stamp-on-
                // first-assistant pass as above so the pending ids
                // get a final attempt before the stream ends.
                if let Some(mut last) = pending_chunk.take() {
                    if pending_request_message_ids.is_some() && last.error.is_none() {
                        for m in last.messages.iter_mut() {
                            if let objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(asst) = m {
                                asst.request_message_ids = pending_request_message_ids.take();
                                break;
                            }
                        }
                    }
                    yield super::StreamItem::Chunk(last);
                }

                // Push the upstream state (carries SDK session_id) onto the
                // continuation BEFORE the early-exit branches. Without this,
                // a turn that ends without calling any tools — common when the
                // agent emits free-form text — drops the session_id, so the
                // next call (e.g. invention's retry-with-error prompt) opens
                // a fresh SDK session and the agent loses memory of the prior
                // turn.
                if let Some(state) = current_state.take() {
                    continuation_items.push(super::ContinuationItem::State(state));
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

                // TODO: return to concurrent dispatch (`join_all`) once
                // we have a way to keep the per-call response order
                // deterministic. The blocker is invention tools that
                // mutate shared state and return order-sensitive values
                // (`AppendTask` returns the new tasks length, etc.):
                // when those run in parallel, the tokio scheduler
                // decides which acquires the state mutex first,
                // shuffling each call's return value and propagating
                // through to the next step's prompt-id-derived mock
                // seed. Possible avenues — server-side ordering by
                // call_id, idempotent-only tools, agent-side
                // parallel-then-canonicalize — all need design.
                //
                // Dispatch tool calls in this turn SEQUENTIALLY in the
                // meantime. We used to `join_all` for latency, but
                // serialising fixes the race by construction. The
                // proxy's per-call latency dominates anyway (the
                // InventionServer's session worker is a single-event
                // loop, so it would have serialised them server-side
                // regardless).
                let conn = mcp_connection
                    .as_ref()
                    .expect("callable extraction returns empty without a connection")
                    .clone();
                let mut results = Vec::with_capacity(callable.len());
                for (call_id, name, args) in &callable {
                    let arguments: Option<
                        indexmap::IndexMap<String, serde_json::Value>,
                    > = serde_json::from_str(args).ok();
                    let res = conn
                        .call_tool_as_message(
                            &objectiveai_sdk::mcp::tool::CallToolRequestParams {
                                name: name.clone(),
                                arguments,
                                _meta: None,
                                task: None,
                            },
                            call_id.clone(),
                        )
                        .await;
                    results.push(res);
                }

                let mut any_invention_tool_called = false;
                for result in results {
                    match result {
                        Ok(tool_msg) => {
                            let idx = continuation_items.len() as u64;
                            let chunk = make_tool_chunk(
                                &id,
                                &agent_instance_hierarchy_header,
                                &agent_id,
                                &agent_full_id,
                                agent_remote.as_ref(),
                                created,
                                upstream_kind,
                                idx,
                                &tool_msg,
                            );
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

                // `disable_tools` is the sentinel the orchestrator hands
                // us to signal "the model has produced what we needed;
                // run the next continuation with tools off so it closes
                // out with a free-form response". We can't tell from
                // here whether any of the tools we just dispatched were
                // invention tools (the proxy hides that), so we let the
                // sentinel decide on its own.
                let _ = &any_invention_tool_called;
                let tools_enabled = !disable_tools.as_ref().is_some_and(|f| f());

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
                        agent_instance_hierarchy_header.as_str(),
                        agent_id.as_str(),
                        agent_full_id.as_str(),
                        agent_remote.as_ref(),
                    )
                    .await
                {
                    Ok(new_stream) => {
                        stream = Box::pin(new_stream);
                    }
                    Err(e) => {
                        use objectiveai_sdk::error::StatusError;
                        let e = map_upstream_err(e);
                        final_error = Some(objectiveai_sdk::error::ResponseError {
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

            // Build response continuation token. The upstream stamps
            // `agent_instance_hierarchy` on the returned continuation
            // itself — no post-stamp from the orchestrator.
            let response_cont = upstream.response_continuation(
                mcp_sessions,
                request_continuation.as_ref(),
                &messages,
                Some(&continuation_items),
                &agent_instance_hierarchy_header,
            );
            let continuation_token: objectiveai_sdk::agent::Continuation = response_cont.into();
            let continuation_token = continuation_token.to_string();

            // Set cancellation error if the stream was cancelled.
            if is_cancelled() && final_error.is_none() {
                final_error = Some(objectiveai_sdk::error::ResponseError::from(
                    &super::Error::StreamCancelled,
                ));
            }

            // Peek the proxy's pending-notifications queue so the
            // caller can tell whether a follow-up continuation would
            // surface queued blocks. Only meaningful when a
            // continuation is also being returned — this site always
            // sets `continuation: Some(_)`, so the peek is
            // unconditionally relevant here. A peek failure is
            // surfaced via the chunk's `error` field only if no prior
            // error already occupies it (the earlier failure is the
            // more important signal).
            //
            // With the CLI→API notify path removed there's nothing to
            // serialize against here — the read of the proxy queue is
            // straight, no lock needed. `messages_queued` simply
            // initializes to `None` and is populated from the peek
            // below.
            let mut messages_queued: Option<bool> = None;
            if let Some(conn) = &mcp_connection {
                match conn.has_pending_notifications().await {
                    Ok(true) => messages_queued = Some(true),
                    Ok(false) => {}
                    Err(error) => {
                        if final_error.is_none() {
                            final_error = Some(
                                objectiveai_sdk::error::ResponseError::from(
                                    &super::Error::McpQueuedNotifications {
                                        url: conn.url.clone(),
                                        error,
                                    },
                                ),
                            );
                        }
                    }
                }
            }

            // Single site for usage, continuation, and error (if a continuation call failed).
            yield super::StreamItem::Chunk(
                objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                    id: id.clone(),
                    agent_instance_hierarchy: agent_instance_hierarchy_header.clone(),
                    agent_id: agent_id.clone(),
                    agent_full_id: agent_full_id.clone(),
                    agent_remote: agent_remote.clone(),
                    created,
                    upstream: upstream_kind,
                    usage: Some(usage),
                    error: final_error,
                    continuation: Some(continuation_token),
                    messages_queued,
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
    agent_instance_hierarchy: &str,
    params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
) -> Option<objectiveai_sdk::agent::completions::request::ResponseFormat> {
    use objectiveai_sdk::agent::completions::request::ResponseFormatParam;
    match params.response_format.as_ref()? {
        ResponseFormatParam::Single(rf) => Some(rf.clone()),
        ResponseFormatParam::PerAgent(map) => map.get(agent_instance_hierarchy).cloned(),
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
    aggregate: &objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
    callable_names: Option<&std::collections::HashSet<String>>,
) -> Vec<(String, String, String)> {
    use objectiveai_sdk::agent::completions::response::streaming::MessageChunk;

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

/// Wrap MCP `ContentBlock`s drained from the proxy at agent init time
/// into a `UserMessage`.
///
/// The blocks are presented to the model as a plain user turn — no
/// `<system-reminder>` wrapper. (The wrapper is reserved for the
/// proxy's tool-response drain path, where notifications surface
/// mid-turn and need the "while you were working" framing.) Init-time
/// notifications are semantically a user message that arrived between
/// turns, so they take the user-message shape directly.
///
/// Delegates to the SDK's [`From<Vec<ContentBlock>> for RichContent`]
/// impl — same mapping (text / image-data-URL / audio direct;
/// `ResourceLink` / `EmbeddedResource` → JSON text), same collapse
/// of all-text inputs into a single `RichContent::Text`. Empty input
/// is the caller's responsibility.
fn build_drain_user_message(
    blocks: Vec<objectiveai_sdk::mcp::tool::ContentBlock>,
) -> objectiveai_sdk::agent::completions::message::UserMessage {
    objectiveai_sdk::agent::completions::message::UserMessage {
        content: blocks.into(),
        name: None,
    }
}

/// Time budget for one `read_message_queue` / `clear_message_queue`
/// round-trip over the WS reverse-attach. Matches
/// [`crate::objectiveai_mcp::handlers::FORWARD_TIMEOUT`]'s shape: long
/// enough that a healthy CLI always answers in time, short enough
/// that a wedged WS doesn't stall the agent loop indefinitely.
const MESSAGE_QUEUE_WS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Issue a `ReadMessageQueue` server-request over the WS reverse-
/// attach and return the joined `(rich_content, ids)` payload.
///
/// The CLI side joins every queued entry into one RichContent
/// (with `"\n\n"` separators between entries) and returns the
/// content-id refs (id + kind) for every consumed
/// `message_queue_contents` row. The envelope carries no
/// `mcp_kind` and the headers map is empty. Failures (channel
/// closed, dropped, timed out, or CLI-side JSON-RPC error) collapse
/// to [`super::Error::MessageQueueRead`].
async fn read_message_queue_via_ws(
    handle: &std::sync::Arc<crate::objectiveai_mcp::ReverseAttachHandle>,
    agent_instance_hierarchy: &str,
) -> Result<
    objectiveai_sdk::client_objectiveai_mcp::server_response::ReadMessageQueueResult,
    super::Error,
> {
    use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};
    let rc = handle.channel();
    let request = server_request::Request {
        id: uuid::Uuid::new_v4().to_string(),
        headers: indexmap::IndexMap::new(),
        payload: server_request::Payload::ReadMessageQueue(
            server_request::ReadMessageQueueRequest {
                agent_instance_hierarchy: agent_instance_hierarchy.to_string(),
            },
        ),
    };
    let rx = crate::objectiveai_mcp::send_server_request(&rc.sink, &rc.pending, request)
        .await
        .map_err(|()| super::Error::MessageQueueRead("reverse channel closed".to_string()))?;
    let response = match tokio::time::timeout(MESSAGE_QUEUE_WS_TIMEOUT, rx).await {
        Ok(Ok(response)) => response,
        Ok(Err(_)) => {
            return Err(super::Error::MessageQueueRead(
                "reverse channel dropped before reply".to_string(),
            ));
        }
        Err(_) => {
            return Err(super::Error::MessageQueueRead(
                "reverse channel timed out".to_string(),
            ));
        }
    };
    match response.payload {
        server_response::Payload::ReadMessageQueue(server_response::JsonRpcResult::Ok {
            result,
        }) => Ok(result),
        server_response::Payload::ReadMessageQueue(server_response::JsonRpcResult::Err {
            code,
            message,
            ..
        }) => Err(super::Error::MessageQueueRead(format!(
            "CLI returned JSON-RPC error {code}: {message}"
        ))),
        other => Err(super::Error::MessageQueueRead(format!(
            "CLI returned wrong variant: {other:?}"
        ))),
    }
}

/// Builds an `AgentCompletionChunk` containing a single tool-response message.
fn make_tool_chunk(
    id: &str,
    agent_instance_hierarchy: &str,
    agent_id: &str,
    agent_full_id: &str,
    agent_remote: Option<&objectiveai_sdk::RemotePath>,
    created: u64,
    upstream: objectiveai_sdk::agent::Upstream,
    index: u64,
    tool_msg: &objectiveai_sdk::agent::completions::message::ToolMessage,
) -> objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
    use objectiveai_sdk::agent::completions::response::streaming::{
        AgentCompletionChunk, MessageChunk,
    };
    use objectiveai_sdk::agent::completions::response::ToolResponse;
    AgentCompletionChunk {
        id: id.to_string(),
        agent_instance_hierarchy: agent_instance_hierarchy.to_string(),
        agent_id: agent_id.to_string(),
        agent_full_id: agent_full_id.to_string(),
        agent_remote: agent_remote.cloned(),
        created,
        upstream,
        messages: vec![MessageChunk::Tool(ToolResponse {
            role: Default::default(),
            index,
            inner: tool_msg.clone(),
            request_message_ids: None,
        })],
        ..Default::default()
    }
}

/// Percent-encode a single path segment for the synthetic per-MCP
/// URLs in `X-MCP-Servers`. Mirrors RFC 3986's `pchar` minus the
/// sub-delims we want literal; deliberately strict so that
/// `owner`/`name`/`version`/`mcp` values containing `/` `?` `#` `&`
/// `=` get encoded and the API's `Path` extractor sees exactly four
/// segments.
fn percent_encode_segment(s: &str) -> String {
    /// `unreserved` + `:@` (the path-segment-internal `pchar` set,
    /// minus sub-delims to keep `&` `=` etc out of segments).
    const SEGMENT: &percent_encoding::AsciiSet =
        &percent_encoding::NON_ALPHANUMERIC
            .remove(b'-')
            .remove(b'.')
            .remove(b'_')
            .remove(b'~')
            .remove(b':')
            .remove(b'@');
    percent_encoding::utf8_percent_encode(s, SEGMENT).to_string()
}

#[cfg(test)]
mod build_drain_user_message_tests {
    use super::build_drain_user_message;
    use objectiveai_sdk::agent::completions::message::{
        RichContent, RichContentPart,
    };
    use objectiveai_sdk::mcp::tool::{
        AudioContent, ContentBlock, ImageContent, TextContent,
    };

    /// All-text input collapses to a single `RichContent::Text` joined
    /// with `\n\n` between blocks. No `<system-reminder>` wrapper —
    /// that's reserved for the proxy's tool-response drain path.
    #[test]
    fn text_only_blocks_collapse_to_joined_text() {
        let msg = build_drain_user_message(vec![
            ContentBlock::Text(TextContent {
                text: "hello".into(),
                annotations: None,
                _meta: None,
            }),
            ContentBlock::Text(TextContent {
                text: "world".into(),
                annotations: None,
                _meta: None,
            }),
        ]);
        assert_eq!(msg.name, None);
        match msg.content {
            RichContent::Text(s) => assert_eq!(s, "hello\n\nworld"),
            other => panic!("expected RichContent::Text, got {other:?}"),
        }
    }

    /// A single text block becomes a plain `Text` (no `Parts` wrapper).
    #[test]
    fn single_text_block_is_plain_text() {
        let msg = build_drain_user_message(vec![ContentBlock::Text(TextContent {
            text: "just one".into(),
            annotations: None,
            _meta: None,
        })]);
        match msg.content {
            RichContent::Text(s) => assert_eq!(s, "just one"),
            other => panic!("expected RichContent::Text, got {other:?}"),
        }
    }

    /// Mixed content (text + image) stays as `Parts` since the image
    /// has to ride alongside the text in a multimodal-aware shape.
    #[test]
    fn mixed_content_becomes_parts() {
        let msg = build_drain_user_message(vec![
            ContentBlock::Text(TextContent {
                text: "look at this".into(),
                annotations: None,
                _meta: None,
            }),
            ContentBlock::Image(ImageContent {
                data: "BASE64DATA".into(),
                mime_type: "image/png".into(),
                annotations: None,
                _meta: None,
            }),
        ]);
        match msg.content {
            RichContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    RichContentPart::Text { text } => assert_eq!(text, "look at this"),
                    other => panic!("expected text part, got {other:?}"),
                }
                match &parts[1] {
                    RichContentPart::ImageUrl { image_url } => {
                        assert_eq!(image_url.url, "data:image/png;base64,BASE64DATA");
                    }
                    other => panic!("expected image part, got {other:?}"),
                }
            }
            other => panic!("expected RichContent::Parts, got {other:?}"),
        }
    }

    /// Audio block round-trips through `InputAudio` part.
    #[test]
    fn audio_block_becomes_input_audio_part() {
        let msg = build_drain_user_message(vec![ContentBlock::Audio(AudioContent {
            data: "AUDIO".into(),
            mime_type: "audio/wav".into(),
            annotations: None,
            _meta: None,
        })]);
        match msg.content {
            RichContent::Parts(parts) => {
                assert_eq!(parts.len(), 1);
                match &parts[0] {
                    RichContentPart::InputAudio { input_audio } => {
                        assert_eq!(input_audio.data, "AUDIO");
                        assert_eq!(input_audio.format, "audio/wav");
                    }
                    other => panic!("expected input_audio part, got {other:?}"),
                }
            }
            other => panic!("expected RichContent::Parts, got {other:?}"),
        }
    }
}
