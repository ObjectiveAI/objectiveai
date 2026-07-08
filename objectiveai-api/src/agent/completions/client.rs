use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};

use crate::ctx;
use futures::StreamExt;

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

/// On agent lock-in, fire a best-effort proxy `drop` for every candidate
/// `response_id` except the winner's. Each candidate's MCP connect was
/// spawned up front, so a non-winner slot may hold a live proxy session
/// (and CLI plugin subprocesses); banning + tearing it down reclaims those
/// as soon as the option is abandoned. Orphan spawns — results discarded.
fn spawn_drop_losers(
    dropper: &objectiveai_mcp_proxy::Dropper,
    response_ids: &[String],
    winner: usize,
) {
    for (j, id) in response_ids.iter().enumerate() {
        if j == winner {
            continue;
        }
        let dropper = dropper.clone();
        let id = id.clone();
        tokio::spawn(async move { dropper.drop(id).await });
    }
}

// ---------------------------------------------------------------------------

pub struct Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG> {
    /// MCP Client
    pub mcp_client: Arc<objectiveai_sdk::mcp::Client>,
    /// Lazy in-process mcp-proxy used for every per-agent MCP connection.
    pub proxy_spawner: Arc<super::ProxyFactory>,
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
        proxy_spawner: Arc<super::ProxyFactory>,
        mcp_authorization: Option<Arc<std::collections::HashMap<String, String>>>,
        retrieve_router: Arc<crate::retrieval::retrieve::Router<RETRG, RETRF, RETRM, CTXEXT>>,
        usage_handler: Arc<CUSG>,
        openrouter: Arc<OPENROUTER>,
        claude_agent_sdk: Arc<CLAUDEAGENTSDK>,
        codex_sdk: Arc<CODEXSDK>,
        mock: Arc<MOCK>,
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
        ctx: ctx::Context<CTXEXT>,
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
    ) -> Result<
        objectiveai_sdk::agent::completions::response::unary::AgentCompletion,
        super::Error,
    > {
        let mut aggregate: Option<
            objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
        > = None;
        let mut stream = self
            .create_streaming_handle_usage(ctx, params, continuation, disable_tools, extra_mcp_servers, extra_mcp_headers, transform_messages)
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
        ctx: ctx::Context<CTXEXT>,
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
                .create_streaming(ctx.clone(), params.clone(), continuation, disable_tools, extra_mcp_servers, extra_mcp_headers, transform_messages)
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
        ctx: ctx::Context<CTXEXT>,
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
        // *without* mutating the agent's own `mcp_servers` config.
        extra_mcp_servers: Vec<super::ExtraMcpServer>,
        // Headers to merge into the per-agent `X-MCP-Headers` map. The
        // proxy forwards these verbatim to every upstream it fans out
        // to.
        extra_mcp_headers: indexmap::IndexMap<String, String>,
        transform_messages: Option<Arc<TransformMessages>>,
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
        // 1. Panic if internal and request continuation upstream types conflict.
        if let (Some(ic), Some(rc)) = (&continuation, &request_continuation) {
            assert_eq!(
                ic.upstream(), rc.upstream(),
                "internal and request continuation upstream types must match"
            );
        }

        // 2. Extract continuation items and upstream type. (The MCP
        // connection is never carried across turns — every turn opens a
        // fresh proxy connection — so there is nothing connection-related
        // to extract here.)
        let cont_upstream = continuation.as_ref().map(|c| c.upstream());
        let (
            mut cont_items_or,
            mut cont_items_cas,
            mut cont_items_cdx,
            mut cont_items_mock,
        ) = match continuation {
            Some(super::Continuation::Openrouter { items, .. }) => {
                (items, vec![], vec![], vec![])
            }
            Some(super::Continuation::ClaudeAgentSdk { items, .. }) => {
                (vec![], items, vec![], vec![])
            }
            Some(super::Continuation::CodexSdk { items, .. }) => {
                (vec![], vec![], items, vec![])
            }
            Some(super::Continuation::Mock { items, .. }) => {
                (vec![], vec![], vec![], items)
            }
            None => (vec![], vec![], vec![], vec![]),
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

        // 5. Boot THIS request's proxy — lazily, once per `Context` — and
        //    kick off one connect per agent in parallel. The proxy lives
        //    on the Context (`proxy_cell`) and dies with it; the factory
        //    (config recipe) + this request's reverse channel + queue
        //    delegate are the boot inputs. Awaiting each `JoinHandle`
        //    inside the per-agent branch later means the round-trips
        //    overlap rather than serializing.
        let proxy_handle = {
            let factory = self.proxy_spawner.clone();
            let reverse_channel = ctx.reverse_channel().cloned();
            let queue_delegate = ctx.queue_delegate();
            ctx.proxy_cell()
                .get_or_try_init(|| async move {
                    factory.boot(reverse_channel, queue_delegate).await
                })
                .await
                .map(Arc::clone)
                .map_err(|e: std::io::Error| super::Error::McpProxyBootstrap(e.to_string()))?
        };
        let proxy_url = proxy_handle.url.clone();
        // This request's dropper: invoked at lock-in (drop the non-selected
        // agent options) and after the winner's final chunk (drop its own
        // response id). See `spawn_drop_losers` + the terminal block in
        // `run_agent_loop`.
        let dropper = proxy_handle.dropper.clone();

        let request_mcp_auth_owned = request_mcp_auth.clone();
        let default_mcp_auth_owned = self.mcp_authorization.clone();
        // Every turn opens a FRESH proxy connection — no MCP session id is
        // resumed (it's no longer carried in the continuation), so the proxy
        // mints a new session and re-dials its upstreams each turn.

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
        //   opens a FRESH proxy connection — no MCP session id is carried
        //   across turns, so the proxy mints a new session and re-dials its
        //   upstreams every turn. Nothing cross-turn keys off the MCP
        //   session id anymore.
        // A CLI-hosted MCP (`client_objectiveai_mcp`) needs this request's
        // reverse channel (the WS). No per-agent registration: the
        // per-request proxy holds the channel directly — there's no
        // response-id routing registry to populate anymore.
        // Laboratories are completion-wide client-side MCP servers: they
        // apply to every agent (and fallback) and, like
        // `client_objectiveai_mcp`, require this request's reverse channel
        // (the WS).
        let has_laboratories =
            params.laboratories.as_ref().is_some_and(|l| !l.is_empty());
        let agent_needs_reverse_attach: Vec<bool> = filtered_agents
            .iter()
            .map(|agent| {
                (agent.base().client_objectiveai_mcp().is_some()
                    || has_laboratories)
                    && ctx.reverse_channel().is_some()
            })
            .collect();

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
                        objectiveai_sdk::mcp::Connection,
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
                // `extra_mcp_servers` (kept out of the agent's own
                // config so its content-hashed ID stays stable across
                // runs).
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
                let mut client_mcp_synthetic_urls: Vec<(
                    String,
                    Option<indexmap::IndexMap<String, Option<String>>>,
                )> = match (
                    needs_reverse_attach,
                    agent.base().client_objectiveai_mcp(),
                ) {
                    (true, Some(client_mcp)) => {
                        let mut out: Vec<(
                            String,
                            Option<indexmap::IndexMap<String, Option<String>>>,
                        )> = Vec::new();
                        let needs_objectiveai = !client_mcp.tools.is_empty()
                            || client_mcp.objectiveai.unwrap_or(false)
                            || client_mcp.plugins.iter().any(|p| p.executable);
                        if needs_objectiveai {
                            out.push(("ws://objectiveai".to_string(), None));
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
                                    format!("ws:///{path}"),
                                    entry.arguments.clone(),
                                ));
                            }
                        }
                        out
                    }
                    _ => Vec::new(),
                };
                // Laboratories: completion-wide client-side MCP servers,
                // appended to every agent (and fallback) when a WS-attached
                // CLI is present. Gated on `needs_reverse_attach` (not on
                // `client_objectiveai_mcp`) — labs apply even when the agent
                // declares no `client_objectiveai_mcp`. Each becomes a
                // synthetic `ws://laboratory/{id}` upstream (no args), flowing through
                // the same URL/header plumbing as the other synthetic URLs.
                //
                // The `ws://laboratory/{id}` URL is just the upstream's address;
                // the proxy must NOT infer laboratory identity by string-parsing
                // it. We carry the typed `Laboratory` explicitly, keyed by URL,
                // in `X-MCP-Laboratories` — the authoritative signal the proxy
                // uses to mark an upstream as a laboratory.
                let mut laboratories_by_url: indexmap::IndexMap<
                    String,
                    objectiveai_sdk::laboratories::Laboratory,
                > = indexmap::IndexMap::new();
                if needs_reverse_attach {
                    if let Some(labs) = &params.laboratories {
                        for lab in labs {
                            let objectiveai_sdk::laboratories::Laboratory::Client(c) = lab;
                            let url = format!(
                                "ws://laboratory/{}",
                                percent_encode_segment(&c.id)
                            );
                            client_mcp_synthetic_urls.push((url.clone(), None));
                            laboratories_by_url.insert(url, lab.clone());
                        }
                    }
                }
                urls.extend(client_mcp_synthetic_urls.iter().map(|(u, _)| u.clone()));

                // No MCP servers → no proxy connection needed for this
                // agent; skip the spawn.
                if urls.is_empty() {
                    return None;
                }

                // Build the per-URL header map sent as `X-MCP-Headers`
                // to the proxy. For each agent-declared server URL,
                // start from the orchestrator-supplied `extra_mcp_headers`
                // and layer on
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
                if let Some(client_mcp) = agent.base().client_objectiveai_mcp() {
                    let objectiveai_url = "ws://objectiveai".to_string();
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
                // Typed laboratory marker (url → Laboratory). Present only when
                // labs are attached; the proxy uses it as the authoritative
                // signal for which upstreams are laboratories.
                if !laboratories_by_url.is_empty() {
                    proxy_request_headers.insert(
                        "X-MCP-Laboratories".to_string(),
                        serde_json::to_string(&laboratories_by_url).unwrap(),
                    );
                }
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
                // Per-agent spawn: connect to the proxy. Every agent's
                // task runs concurrently with every other agent's, so
                // the proxy `initialize` round-trips fan out in
                // parallel.
                //
                // Plugin MCP upstreams are NOT dialed here — the CLI
                // dials them inside its `initialize` handler.
                //
                // `session_id` is `None`: we never resume a prior proxy
                // session, so the proxy mints a fresh one every turn.
                //
                // No `list_tools` round-trip: presence of the agent's
                // declared `client_objectiveai_mcp` tools/plugins is
                // enforced by objectiveai-mcp itself, which validates
                // the `X-OBJECTIVEAI-MCP-{TOOLS,PLUGINS}` filter sets by
                // `{owner, name, version}` against the installed
                // manifest and errors at connect time on any miss. The
                // server must NOT re-validate against the upstream's
                // advertised tool names — that would wrongly couple it
                // to the client's tool-name scheme.
                //
                // Error type is `Arc<mcp::Error>` to match the SDK's
                // shared-ref error shape uniformly.
                Some(tokio::spawn(async move {
                    let conn = mcp_client
                        .connect(proxy_url, None, Some(proxy_request_headers))
                        .await
                        .map_err(std::sync::Arc::new)?;
                    Ok::<_, std::sync::Arc<objectiveai_sdk::mcp::Error>>(conn)
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
                        objectiveai_sdk::mcp::Connection,
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
        // Every candidate's response id, captured before the loop borrows
        // `attempts` mutably — used to drop the non-winner options at
        // lock-in (`response_ids` itself was moved into `attempts` above).
        let all_response_ids: Vec<String> = attempts.iter().map(|a| a.id.clone()).collect();
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
                // handles here is cheap on later retry iterations.
                // objectiveai-mcp already enforced the agent's declared
                // tool/plugin set against its installed manifest at
                // connect time, so there is nothing to re-validate.
                if !attempt_connect_done[idx] {
                    attempt_connect_done[idx] = true;
                    if let Some(handle) = attempt.connect_handle.take() {
                        match handle.await.unwrap() {
                            Ok(conn) => attempt_connections[idx] = Some(conn),
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
                    || attempt.agent.base().client_objectiveai_mcp().is_some()
                    || has_laboratories;
                let mcp_connection: Option<objectiveai_sdk::mcp::Connection> =
                    attempt_connections[idx].clone();
                if agent_needs_mcp && mcp_connection.is_none() {
                    if attempt.agent.base().client_objectiveai_mcp().is_some()
                        && ctx.reverse_attach().is_none()
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
                            let rc = match &request_continuation {
                                Some(objectiveai_sdk::agent::Continuation::Openrouter(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.openrouter.clone(), or_agent, rc, &params, mcp_connection.clone(),
                                ctx.reverse_attach().cloned(),
                                ctx.queue_delegate(),
                                &mut cont_items_or, &attempt.id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                {
                                    let agent_instance_hierarchy = attempt.agent_instance_hierarchy.clone();
                                    move |items| super::Continuation::Openrouter {
                                        items, agent_instance_hierarchy,
                                    }
                                },
                                |e| super::Error::UpstreamOpenrouter(Box::new(e)),
                                objectiveai_sdk::agent::InlineAgentRef::Openrouter(&or_agent.base),
                                disable_tools.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                attempt.agent_instance_hierarchy.as_str(),
                                attempt.agent.id(),
                                agent_full_id.as_str(),
                                agent_remote.as_ref(),
                            ).await {
                                Ok(stream) => {
                                    // Lock-in: this agent yielded its first
                                    // chunk; drop every other candidate.
                                    spawn_drop_losers(&dropper, &all_response_ids, idx);
                                    return Ok(stream);
                                }
                                Err(e) => e,
                            }
                        }
                        objectiveai_sdk::agent::InlineAgent::ClaudeAgentSdk(cas_agent) => {
                            let rc = match &request_continuation {
                                Some(objectiveai_sdk::agent::Continuation::ClaudeAgentSdk(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.claude_agent_sdk.clone(), cas_agent, rc, &params, mcp_connection.clone(),
                                ctx.reverse_attach().cloned(),
                                ctx.queue_delegate(),
                                &mut cont_items_cas, &attempt.id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                {
                                    let agent_instance_hierarchy = attempt.agent_instance_hierarchy.clone();
                                    move |items| super::Continuation::ClaudeAgentSdk {
                                        items, agent_instance_hierarchy,
                                    }
                                },
                                |e| super::Error::UpstreamClaudeAgentSdk(Box::new(e)),
                                objectiveai_sdk::agent::InlineAgentRef::ClaudeAgentSdk(&cas_agent.base),
                                disable_tools.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                attempt.agent_instance_hierarchy.as_str(),
                                attempt.agent.id(),
                                agent_full_id.as_str(),
                                agent_remote.as_ref(),
                            ).await {
                                Ok(stream) => {
                                    // Lock-in: this agent yielded its first
                                    // chunk; drop every other candidate.
                                    spawn_drop_losers(&dropper, &all_response_ids, idx);
                                    return Ok(stream);
                                }
                                Err(e) => e,
                            }
                        }
                        objectiveai_sdk::agent::InlineAgent::CodexSdk(cdx_agent) => {
                            let rc = match &request_continuation {
                                Some(objectiveai_sdk::agent::Continuation::CodexSdk(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.codex_sdk.clone(), cdx_agent, rc, &params, mcp_connection.clone(),
                                ctx.reverse_attach().cloned(),
                                ctx.queue_delegate(),
                                &mut cont_items_cdx, &attempt.id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                {
                                    let agent_instance_hierarchy = attempt.agent_instance_hierarchy.clone();
                                    move |items| super::Continuation::CodexSdk {
                                        items, agent_instance_hierarchy,
                                    }
                                },
                                |e| super::Error::UpstreamCodexSdk(Box::new(e)),
                                objectiveai_sdk::agent::InlineAgentRef::CodexSdk(&cdx_agent.base),
                                disable_tools.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                attempt.agent_instance_hierarchy.as_str(),
                                attempt.agent.id(),
                                agent_full_id.as_str(),
                                agent_remote.as_ref(),
                            ).await {
                                Ok(stream) => {
                                    // Lock-in: this agent yielded its first
                                    // chunk; drop every other candidate.
                                    spawn_drop_losers(&dropper, &all_response_ids, idx);
                                    return Ok(stream);
                                }
                                Err(e) => e,
                            }
                        }
                        objectiveai_sdk::agent::InlineAgent::Mock(mock_agent) => {
                            let rc = match &request_continuation {
                                Some(objectiveai_sdk::agent::Continuation::Mock(c)) => Some(c),
                                _ => None,
                            };
                            match self.run_agent_loop(
                                self.mock.clone(), mock_agent, rc, &params, mcp_connection.clone(),
                                ctx.reverse_attach().cloned(),
                                ctx.queue_delegate(),
                                &mut cont_items_mock, &attempt.id, created,
                                *byok_attempt, ctx.cost_multiplier,
                                {
                                    let agent_instance_hierarchy = attempt.agent_instance_hierarchy.clone();
                                    move |items| super::Continuation::Mock {
                                        items, agent_instance_hierarchy,
                                    }
                                },
                                |e| super::Error::UpstreamMock(Box::new(e)),
                                objectiveai_sdk::agent::InlineAgentRef::Mock(&mock_agent.base),
                                disable_tools.clone(),
                                agent_transform,
                                make_is_cancelled(),
                                attempt.agent_instance_hierarchy.as_str(),
                                attempt.agent.id(),
                                agent_full_id.as_str(),
                                agent_remote.as_ref(),
                            ).await {
                                Ok(stream) => {
                                    // Lock-in: this agent yielded its first
                                    // chunk; drop every other candidate.
                                    spawn_drop_losers(&dropper, &all_response_ids, idx);
                                    return Ok(stream);
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
                return Err(super::Error::NoAgentsResolved);
            }
            use backoff::backoff::Backoff;
            match backoff.next_backoff() {
                Some(d) => tokio::time::sleep(d).await,
                None => {
                    return Err(if errors.len() == 1 {
                        errors.into_iter().next().unwrap()
                    } else {
                        super::Error::MultipleErrors(errors)
                    });
                }
            }
        }
    }

    /// Creates an upstream stream and runs the tool-calling loop.
    ///
    /// 1. Calls `upstream.create()` with `first_chunk_timeout`.
    /// 2. Returns a stream that yields chunks as they arrive, executes
    ///    callable tools (MCP), and re-invokes the upstream for each
    ///    continuation until no more callable tool calls remain.
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
        queue_delegate: Arc<super::queue_delegate::ApiQueueDelegate>,
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
        // On resume, SKIP the personality merge (system_prompt / prefix /
        // suffix are already in the prior turn's accumulated state, so re-
        // merging would snowball a duplicate prefix every turn) — but KEEP
        // `params.messages`. That field carries THIS turn's new user content;
        // dropping it (the old `Vec::new()`) silently lost the user's message
        // on every resume. `merged_messages` is the only thing that injects
        // the personality, so omitting it is sufficient to avoid duplication.
        let mut messages = if resuming {
            params.messages.clone()
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
                // The queued content leads as one user turn, then any
                // caller-supplied content follows. The agent's system prompt
                // is no longer carried as a conversation message, so the
                // queued message simply goes at the front.
                messages.insert(
                    0,
                    Message::User(UserMessage {
                        content: rich_content,
                    }),
                );
            }
        }

        // Register with the in-process MCP-proxy queue delegate
        // for the lifetime of this loop. The proxy calls
        // `read_pending_blocks` on every tool response; the
        // delegate routes by AIH to the per-loop state we just
        // seeded with `queue_ids_to_clear` (the startup snapshot
        // that we already stamped on the first assistant chunk).
        // Subsequent reads filter against this confirmed set so
        // the proxy never re-surfaces a row we already injected
        // upfront. Registration is conditional on a
        // `reverse_attach` handle being available — without one
        // the delegate can't reach back to the CLI to read the
        // queue, so the proxy gets a no-op when it calls in.
        let delegate_guard = if let Some(handle) = &reverse_attach {
            let delegate = queue_delegate.clone();
            delegate
                .register(
                    agent_instance_hierarchy_header.to_string(),
                    handle.clone(),
                    queue_ids_to_clear.clone(),
                )
                .await;
            Some(DelegateUnregisterGuard {
                delegate,
                aih: agent_instance_hierarchy_header.to_string(),
            })
        } else {
            None
        };

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
        // (proxy-routed MCP calls) from tool calls the upstream
        // encodes for its own reasons (response_format, etc.).
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
        // Capture into the stream. The Drop on `_delegate_guard`
        // fires when the stream future is dropped (natural
        // end-of-stream OR early cancel), spawning the async
        // unregister — pending tokens that never got confirmed
        // (and therefore weren't really delivered) drop with the
        // state, so the queue rows re-issue on the next loop.
        let _delegate_guard = delegate_guard;
        let queue_delegate_for_stream = queue_delegate.clone();

        Ok(Box::pin(async_stream::stream! {
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
                        Ok(Some(super::StreamItem::Chunk(mut chunk))) => {
                            // Identity (`agent_instance_hierarchy`,
                            // `agent_id`, `agent_full_id`, `agent_remote`)
                            // is stamped at the upstream-client level
                            // when each chunk is constructed — no need
                            // to re-stamp here.
                            // Empty-string reasoning is a no-content
                            // marker (thinking-block starts, redacted /
                            // omitted reasoning) — normalize to None
                            // HERE, at the downstream client, so empty
                            // reasoning is impossible on the wire, in
                            // accumulators, and in log rows, whatever
                            // the upstream emitted.
                            // Import usage from assistant response chunks.
                            for msg in &mut chunk.messages {
                                if let objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Assistant(asst) = msg {
                                    if asst.reasoning.as_deref() == Some("") {
                                        asst.reasoning = None;
                                    }
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
                // next call opens a fresh SDK session and the agent loses
                // memory of the prior turn.
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
                // deterministic. The blocker is tools that mutate
                // shared state and return order-sensitive values:
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
                // proxy's per-call latency dominates anyway.
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

                for ((call_id, ..), result) in callable.iter().zip(results) {
                    match result {
                        Ok(tool_msg) => {
                            let idx = continuation_items.len() as u64;
                            // Regex-scan the tool message for the
                            // proxy's confirmation prefix. Each
                            // captured token resolves to the IDs the
                            // delegate speculatively issued in
                            // that batch; `confirm()` promotes them
                            // from pending→confirmed and we stamp
                            // them onto the ToolResponse so the
                            // downstream LogWriter logs the
                            // delivery via `MessageQueueContent` row
                            // writes. Tokens we never see (proxy
                            // didn't append a prefix this turn,
                            // tool message truncated, etc.) stay
                            // pending until `unregister` drops
                            // them — those rows re-deliver next
                            // loop.
                            let request_message_ids = scan_and_confirm_tokens(
                                &queue_delegate_for_stream,
                                &agent_instance_hierarchy_header,
                                &tool_msg.content,
                            )
                            .await;
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
                                request_message_ids,
                            );
                            if let Some(ref mut agg) = aggregate {
                                agg.push(&chunk);
                            }
                            yield super::StreamItem::Chunk(chunk);
                            continuation_items
                                .push(super::ContinuationItem::ToolMessage(tool_msg));
                        }
                        Err(e) => {
                            // The dispatch itself failed — almost always a
                            // timeout under load, but also proxy
                            // -32601/-32603 or a transport drop. Previously
                            // this set `had_error` and broke, silently
                            // dropping the tool call: the assistant's
                            // `tool_call` was left with no `tool_response`
                            // and the whole turn aborted, leaving an
                            // orphaned call the next continuation couldn't
                            // resolve. Forward the failure to the agent
                            // instead — synthesize an error `tool_response`
                            // so the call is always answered and the loop
                            // continues, exactly as an upstream
                            // `is_error: true` result would. The model sees
                            // the failure text and can retry or move on.
                            let tool_msg =
                                objectiveai_sdk::agent::completions::message::ToolMessage {
                                    content:
                                        objectiveai_sdk::agent::completions::message::RichContent::Text(
                                            format!("tool call failed: {e}"),
                                        ),
                                    tool_call_id: call_id.clone(),
                                    metadata: None,
                                };
                            let idx = continuation_items.len() as u64;
                            // No queue-delegate confirmation on a failed
                            // dispatch — nothing was delivered.
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
                                None,
                            );
                            if let Some(ref mut agg) = aggregate {
                                agg.push(&chunk);
                            }
                            yield super::StreamItem::Chunk(chunk);
                            continuation_items
                                .push(super::ContinuationItem::ToolMessage(tool_msg));
                        }
                    }
                }

                // `disable_tools` is the sentinel the orchestrator hands
                // us to signal "the model has produced what we needed;
                // run the next continuation with tools off so it closes
                // out with a free-form response".
                let tools_enabled = !disable_tools.as_ref().is_some_and(|f| f());

                if had_error {
                    break;
                }

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
                        mcp_connection.clone(),
                        Some(&continuation_items),
                        byok.as_deref(),
                        cost_multiplier,
                        tools_enabled,
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

            // Build response continuation token. The upstream stamps
            // `agent_instance_hierarchy` on the returned continuation
            // itself — no post-stamp from the orchestrator. No MCP session
            // id is carried across turns (every turn connects fresh).
            let response_cont = upstream.response_continuation(
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
                    ..Default::default()
                },
            );
            let cont = wrap_continuation(continuation_items);
            yield super::StreamItem::State(cont);
        }))
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

// The time budget for one `read_message_queue` round-trip over the
// WS reverse-attach is the shared configured reverse-channel budget,
// carried by the handle itself —
// `ReverseAttachHandle::reverse_channel_timeout`.

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
    let response = match tokio::time::timeout(handle.reverse_channel_timeout(), rx).await {
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

/// Builds an `AgentCompletionChunk` containing a single
/// tool-response message. `request_message_ids` is `Some` when
/// the run-loop's prefix-token scan over `tool_msg.content`
/// confirmed delivery for one or more `message_queue_contents.id`s
/// (the `ApiQueueDelegate::confirm` return value); `None` otherwise.
#[allow(clippy::too_many_arguments)]
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
    request_message_ids: Option<Vec<i64>>,
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
            request_message_ids,
        })],
        ..Default::default()
    }
}

/// Drop-fires an async `unregister` on the embedded queue
/// delegate when the agent loop's stream future drops. Pending
/// tokens that never got confirmed go with it; their ids
/// re-deliver on the next loop.
struct DelegateUnregisterGuard {
    delegate: std::sync::Arc<super::ApiQueueDelegate>,
    aih: String,
}

impl Drop for DelegateUnregisterGuard {
    fn drop(&mut self) {
        let delegate = self.delegate.clone();
        let aih = std::mem::take(&mut self.aih);
        tokio::spawn(async move {
            delegate.unregister(&aih).await;
        });
    }
}

/// Scan every text part of a `ToolMessage`'s content for the
/// `<system-reminder>` confirmation prefix the MCP proxy
/// embedded (one token per delegate batch), call
/// `ApiQueueDelegate::confirm` on each captured token, and
/// concatenate the returned ids in scan order. Returns `None`
/// when nothing matched or no delegate is wired in.
async fn scan_and_confirm_tokens(
    delegate: &super::ApiQueueDelegate,
    aih: &str,
    content: &objectiveai_sdk::agent::completions::message::RichContent,
) -> Option<Vec<i64>> {
    use objectiveai_sdk::agent::completions::message::{RichContent, RichContentPart};
    let texts: Vec<&str> = match content {
        RichContent::Text(t) => vec![t.as_str()],
        RichContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                RichContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect(),
    };
    let mut all_ids: Vec<i64> = Vec::new();
    for text in texts {
        for token in
            objectiveai_sdk::mcp::queue_notification::extract_tokens(text)
        {
            let ids = delegate.confirm(aih, &token).await;
            all_ids.extend(ids);
        }
    }
    if all_ids.is_empty() { None } else { Some(all_ids) }
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
