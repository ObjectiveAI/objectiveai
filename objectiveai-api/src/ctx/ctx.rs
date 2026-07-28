//! Request context containing per-request state and caches.

use dashmap::DashMap;
use futures::future::Shared;
use futures::FutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// The three per-1-SECOND per-upstream duration billing rates carried by
/// [`Context`], bundled so the server initializer can define them as one
/// `const`. See the field docs on [`Context`] for the exact math and rules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DurationCosts {
    /// Rate for OpenRouter upstream wall time.
    pub openrouter_duration_cost: rust_decimal::Decimal,
    /// Rate for Claude Agent SDK upstream wall time.
    pub claude_agent_sdk_duration_cost: rust_decimal::Decimal,
    /// Rate for Codex SDK upstream wall time.
    pub codex_sdk_duration_cost: rust_decimal::Decimal,
    /// Rate for Script upstream wall time (client-side script
    /// execution over the reverse channel).
    pub script_duration_cost: rust_decimal::Decimal,
}

impl DurationCosts {
    /// All rates zero — duration is tracked but not billed.
    pub const ZERO: DurationCosts = DurationCosts {
        openrouter_duration_cost: rust_decimal::Decimal::ZERO,
        claude_agent_sdk_duration_cost: rust_decimal::Decimal::ZERO,
        codex_sdk_duration_cost: rust_decimal::Decimal::ZERO,
        script_duration_cost: rust_decimal::Decimal::ZERO,
    };
}

/// Per-request context containing user-specific state and deduplication caches.
///
/// The context is generic over `CTXEXT`, allowing custom extensions for
/// different deployment scenarios (e.g., different BYOK providers).
///
/// # Caches
///
/// The caches deduplicate concurrent fetches for the same resource within a request.
/// When multiple parts of a request need the same swarm or agent,
/// only one fetch is performed and the result is shared.
#[derive(Debug)]
pub struct Context<CTXEXT> {
    /// Custom context extension (e.g., for BYOK keys).
    pub ext: Arc<CTXEXT>,
    /// Multiplier applied to costs for this request.
    pub cost_multiplier: rust_decimal::Decimal,
    /// Per-1-SECOND cost of OpenRouter upstream wall time
    /// (`usage.upstream_duration_ms.openrouter`), charged raw (no
    /// `cost_multiplier`), BYOK included.
    pub openrouter_duration_cost: rust_decimal::Decimal,
    /// Per-1-SECOND cost of Claude Agent SDK upstream wall time
    /// (`usage.upstream_duration_ms.claude_agent_sdk`).
    pub claude_agent_sdk_duration_cost: rust_decimal::Decimal,
    /// Per-1-SECOND cost of Codex SDK upstream wall time
    /// (`usage.upstream_duration_ms.codex_sdk`).
    pub codex_sdk_duration_cost: rust_decimal::Decimal,
    /// Per-1-SECOND cost of Script upstream wall time
    /// (`usage.upstream_duration_ms.script`).
    pub script_duration_cost: rust_decimal::Decimal,
    /// Whether to suppress output (eprintln, logging, etc).
    pub suppress_output: bool,
    /// Per-request ObjectiveAI authorization token.
    objectiveai_authorization: Option<Arc<String>>,
    /// Per-request OpenRouter authorization token.
    openrouter_authorization: Option<Arc<String>>,
    /// Per-request GitHub authorization token.
    github_authorization: Option<Arc<String>>,
    /// Per-request MCP authorization headers.
    mcp_authorization: Option<Arc<HashMap<String, String>>>,
    /// Per-request MCP CALL budget (`X-MCP-CALL-TIMEOUT`, integer ms):
    /// applied to every MCP call the request's proxy makes (HTTP and
    /// ws:// upstreams alike). Absent or unparseable ⇒ `None` ⇒ NO call
    /// timeout. Never applies to connects or laboratory transfers.
    mcp_call_timeout_ms: Option<u64>,
    /// Per-request caller-supplied agent id (`X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`).
    /// Plays the role of the *parent* when composing the agent id we
    /// forward to the MCP proxy inside agent completions.
    agent_instance_hierarchy: Option<Arc<String>>,
    /// Handle for registering per-agent `response_id`s against the
    /// current WS reverse channel. Set on WS-attached requests by the
    /// streaming handlers; `None` on HTTP/SSE. Many ids may register
    /// against the same handle — one per swarm agent that declares
    /// client-side MCP — all cleaned up when the owning
    /// [`crate::streaming_ws::ReverseAttachGuard`] drops at WS close.
    reverse_attach: Option<Arc<crate::streaming_ws::ReverseAttachHandle>>,
    /// Per-request reverse channel for `ws://` MCP upstreams. Set by the
    /// streaming WS handler (`with_reverse_channel`); `None` on HTTP/SSE
    /// (no CLI attached → HTTP MCP upstreams only). Handed to this
    /// request's proxy at boot so `client://` upstreams speak the
    /// reverse-channel protocol directly over the WS.
    reverse_channel: Option<objectiveai_mcp_proxy::ReverseChannel>,
    /// This request's in-process MCP proxy, lazily booted on first MCP
    /// need (see `agent::completions::Client::create_streaming`). Held by
    /// `Arc<OnceCell>` so the proxy's `axum::serve` task is cancelled (via
    /// the `ProxyHandle`'s `DropGuard`) when the context's last clone
    /// drops — i.e. the proxy dies with the request.
    proxy: Arc<OnceCell<Arc<crate::agent::completions::ProxyHandle>>>,
    /// Per-request queue-read delegate the proxy uses to splice pending
    /// `message_queue` content onto tool responses; also driven by
    /// `run_agent_loop` (register/confirm/unregister). Per-request so its
    /// per-AIH state dies with the context.
    queue_delegate: Arc<crate::agent::completions::ApiQueueDelegate>,
    /// Cached resolved OpenRouter authorization (self + ext).
    openrouter_authorization_cached: Arc<OnceCell<Option<Arc<String>>>>,
    /// Cached resolved GitHub authorization (self + ext).
    github_authorization_cached: Arc<OnceCell<Option<Arc<String>>>>,
    /// Cached resolved MCP authorization (self + ext merged).
    mcp_authorization_cached: Arc<OnceCell<Option<Arc<HashMap<String, String>>>>>,
    /// Cancellation signal — cancelled when the client disconnects.
    /// A `CancellationToken` rather than an `AtomicBool` so consumers
    /// can both peek synchronously (`is_cancelled`) and await the
    /// signal (`cancellation_token().cancelled()`); clones share one
    /// linked state.
    cancelled: tokio_util::sync::CancellationToken,
    /// Cache for agent fetches, keyed by RemotePath.
    agent_cache: Arc<
        DashMap<
            objectiveai_sdk::RemotePath,
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>,
                        objectiveai_sdk::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
    /// Cache for swarm fetches, keyed by RemotePath.
    swarm_cache: Arc<
        DashMap<
            objectiveai_sdk::RemotePath,
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<objectiveai_sdk::swarm::RemoteSwarmBase>,
                        objectiveai_sdk::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
    /// Cache for function fetches, keyed by RemotePath.
    function_cache: Arc<
        DashMap<
            objectiveai_sdk::RemotePath,
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<objectiveai_sdk::functions::FullRemoteFunction>,
                        objectiveai_sdk::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
    /// Cache for profile fetches, keyed by RemotePath.
    profile_cache: Arc<
        DashMap<
            objectiveai_sdk::RemotePath,
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<objectiveai_sdk::functions::RemoteProfile>,
                        objectiveai_sdk::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
    /// Cache for resolve_latest fetches, keyed by RemotePathCommitOptional.
    remote_latest_cache: Arc<
        DashMap<
            objectiveai_sdk::RemotePathCommitOptional,
            Shared<
                tokio::sync::oneshot::Receiver<
                    Result<
                        Option<objectiveai_sdk::RemotePath>,
                        objectiveai_sdk::error::ResponseError,
                    >,
                >,
            >,
        >,
    >,
}

impl<CTXEXT> Clone for Context<CTXEXT> {
    fn clone(&self) -> Self {
        Self {
            ext: self.ext.clone(),
            cost_multiplier: self.cost_multiplier,
            openrouter_duration_cost: self.openrouter_duration_cost,
            claude_agent_sdk_duration_cost: self.claude_agent_sdk_duration_cost,
            codex_sdk_duration_cost: self.codex_sdk_duration_cost,
            script_duration_cost: self.script_duration_cost,
            suppress_output: self.suppress_output,
            objectiveai_authorization: self.objectiveai_authorization.clone(),
            openrouter_authorization: self.openrouter_authorization.clone(),
            github_authorization: self.github_authorization.clone(),
            mcp_authorization: self.mcp_authorization.clone(),
            mcp_call_timeout_ms: self.mcp_call_timeout_ms,
            agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
            reverse_attach: self.reverse_attach.clone(),
            reverse_channel: self.reverse_channel.clone(),
            proxy: self.proxy.clone(),
            queue_delegate: self.queue_delegate.clone(),
            openrouter_authorization_cached: self.openrouter_authorization_cached.clone(),
            github_authorization_cached: self.github_authorization_cached.clone(),
            mcp_authorization_cached: self.mcp_authorization_cached.clone(),
            cancelled: self.cancelled.clone(),
            swarm_cache: self.swarm_cache.clone(),
            agent_cache: self.agent_cache.clone(),
            function_cache: self.function_cache.clone(),
            profile_cache: self.profile_cache.clone(),
            remote_latest_cache: self.remote_latest_cache.clone(),
        }
    }
}

impl<CTXEXT> Context<CTXEXT> {
    /// Returns whether this context has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.is_cancelled()
    }

    /// Marks this context as cancelled.
    pub fn cancel(&self) {
        self.cancelled.cancel();
    }

    /// A clone of this request's cancellation token — the awaitable
    /// side of [`Self::is_cancelled`] (`token.cancelled().await`
    /// resolves when [`Self::cancel`] fires, immediately if it already
    /// has). Clones are linked to the same state.
    pub fn cancellation_token(&self) -> tokio_util::sync::CancellationToken {
        self.cancelled.clone()
    }

    /// Creates a new context by extracting authorization headers from the request.
    ///
    /// For each header, checks the `X-` prefixed variant first, then falls back
    /// to the non-prefixed variant:
    /// - `X-OPENROUTER-AUTHORIZATION` / `OPENROUTER-AUTHORIZATION`: OpenRouter API key
    /// - `X-GITHUB-AUTHORIZATION` / `GITHUB-AUTHORIZATION`: GitHub token
    /// - `X-MCP-AUTHORIZATION` / `MCP-AUTHORIZATION`: JSON-encoded `HashMap<String, String>`
    /// - `X-OBJECTIVEAI-AUTHORIZATION` / `AUTHORIZATION`: ObjectiveAI API key
    pub fn new(
        ext: Arc<CTXEXT>,
        cost_multiplier: rust_decimal::Decimal,
        duration_costs: DurationCosts,
        suppress_output: bool,
        headers: &axum::http::HeaderMap,
    ) -> Self {
        let objectiveai_authorization = headers
            .get("X-OBJECTIVEAI-AUTHORIZATION")
            .or_else(|| headers.get("OBJECTIVEAI-AUTHORIZATION"))
            .or_else(|| headers.get("AUTHORIZATION"))
            .and_then(|v| v.to_str().ok())
            .map(|s| Arc::new(s.to_owned()));

        let openrouter_authorization = headers
            .get("X-OPENROUTER-AUTHORIZATION")
            .or_else(|| headers.get("OPENROUTER-AUTHORIZATION"))
            .and_then(|v| v.to_str().ok())
            .map(|s| Arc::new(s.to_owned()));

        let github_authorization = headers
            .get("X-GITHUB-AUTHORIZATION")
            .or_else(|| headers.get("GITHUB-AUTHORIZATION"))
            .and_then(|v| v.to_str().ok())
            .map(|s| Arc::new(s.to_owned()));

        let mcp_authorization = headers
            .get("X-MCP-AUTHORIZATION")
            .or_else(|| headers.get("MCP-AUTHORIZATION"))
            .and_then(|v| v.to_str().ok())
            .and_then(|s| serde_json::from_str::<HashMap<String, String>>(s).ok())
            .map(Arc::new);

        // Absent OR unparseable ⇒ None ⇒ no MCP call timeout — the
        // `.ok()` chain gives the parse-failure fallback for free.
        let mcp_call_timeout_ms = headers
            .get("X-MCP-CALL-TIMEOUT")
            .or_else(|| headers.get("MCP-CALL-TIMEOUT"))
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());

        let agent_instance_hierarchy = headers
            .get("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY")
            .or_else(|| headers.get("OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY"))
            .and_then(|v| v.to_str().ok())
            .map(|s| Arc::new(s.to_owned()));

        Self {
            ext,
            cost_multiplier,
            openrouter_duration_cost: duration_costs.openrouter_duration_cost,
            claude_agent_sdk_duration_cost: duration_costs.claude_agent_sdk_duration_cost,
            codex_sdk_duration_cost: duration_costs.codex_sdk_duration_cost,
            script_duration_cost: duration_costs.script_duration_cost,
            suppress_output,
            openrouter_authorization,
            github_authorization,
            mcp_authorization,
            mcp_call_timeout_ms,
            objectiveai_authorization,
            agent_instance_hierarchy,
            reverse_attach: None,
            reverse_channel: None,
            proxy: Arc::new(OnceCell::new()),
            queue_delegate: Arc::new(
                crate::agent::completions::ApiQueueDelegate::new(),
            ),
            openrouter_authorization_cached: Arc::new(OnceCell::new()),
            github_authorization_cached: Arc::new(OnceCell::new()),
            mcp_authorization_cached: Arc::new(OnceCell::new()),
            cancelled: tokio_util::sync::CancellationToken::new(),
            swarm_cache: Arc::new(DashMap::new()),
            agent_cache: Arc::new(DashMap::new()),
            function_cache: Arc::new(DashMap::new()),
            profile_cache: Arc::new(DashMap::new()),
            remote_latest_cache: Arc::new(DashMap::new()),
        }
    }
}

impl<CTXEXT> Context<CTXEXT> {
    /// Returns the per-request ObjectiveAI authorization token, if present.
    pub fn objectiveai_authorization(&self) -> Option<&Arc<String>> {
        self.objectiveai_authorization.as_ref()
    }

    /// Returns the caller-supplied agent id from `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`,
    /// if present. This is the *parent* prefix used when composing the
    /// agent id we forward to the MCP proxy.
    pub fn agent_instance_hierarchy(&self) -> Option<&str> {
        self.agent_instance_hierarchy.as_deref().map(|s| s.as_str())
    }

    /// The per-request MCP CALL budget from `X-MCP-CALL-TIMEOUT`
    /// (integer ms). `None` (absent or unparseable) ⇒ the proxy applies
    /// NO call timeout.
    pub fn mcp_call_timeout_ms(&self) -> Option<u64> {
        self.mcp_call_timeout_ms
    }

    /// Returns the shared reverse-attach handle for registering
    /// per-agent `response_id`s against the current WS, if a
    /// streaming WS handler stamped one.
    pub fn reverse_attach(
        &self,
    ) -> Option<&Arc<crate::streaming_ws::ReverseAttachHandle>> {
        self.reverse_attach.as_ref()
    }

    /// Stamps the shared reverse-attach handle on the context.
    /// Returns the modified context for chaining.
    pub fn with_reverse_attach(
        mut self,
        handle: Arc<crate::streaming_ws::ReverseAttachHandle>,
    ) -> Self {
        self.reverse_attach = Some(handle);
        self
    }

    /// Stamps the per-request reverse channel (for `ws://` MCP upstreams)
    /// on the context. Set by the streaming WS handler. Returns the
    /// modified context for chaining.
    pub fn with_reverse_channel(
        mut self,
        channel: objectiveai_mcp_proxy::ReverseChannel,
    ) -> Self {
        self.reverse_channel = Some(channel);
        self
    }

    /// The per-request reverse channel, if a WS handler stamped one.
    pub fn reverse_channel(&self) -> Option<&objectiveai_mcp_proxy::ReverseChannel> {
        self.reverse_channel.as_ref()
    }

    /// This request's lazily-booted proxy cell. `create_streaming` calls
    /// `get_or_try_init` on it with a boot closure built from the
    /// `Client`'s proxy factory + this context's reverse channel +
    /// queue delegate. The `ProxyHandle` (and its serve-task `DropGuard`)
    /// lives here, so the proxy dies when the context's last clone drops.
    pub fn proxy_cell(&self) -> &OnceCell<Arc<crate::agent::completions::ProxyHandle>> {
        &self.proxy
    }

    /// This request's queue-read delegate (per-AIH state, request-scoped).
    pub fn queue_delegate(
        &self,
    ) -> Arc<crate::agent::completions::ApiQueueDelegate> {
        self.queue_delegate.clone()
    }
}

/// Per-request fetch dedup: check the in-memory `DashMap`, and on a miss
/// run `fetch` once, sharing its result across concurrent callers for the
/// same key within this request. All results (including `None`/`Err`) are
/// memoized for the request's lifetime.
async fn cached_get_or_fetch<K, V, F, Fut>(
    cache: &DashMap<
        K,
        Shared<tokio::sync::oneshot::Receiver<Result<Option<V>, objectiveai_sdk::error::ResponseError>>>,
    >,
    key: K,
    fetch: F,
) -> Result<Option<V>, objectiveai_sdk::error::ResponseError>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Option<V>, objectiveai_sdk::error::ResponseError>> + Send,
{
    let shared = cache
        .entry(key)
        .or_insert_with(|| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::spawn(async move {
                let _ = tx.send(fetch().await);
            });
            rx.shared()
        })
        .clone();
    shared.await.unwrap()
}

impl<CTXEXT> Context<CTXEXT> {
    pub async fn cached_agent<F, Fut>(
        &self,
        key: objectiveai_sdk::RemotePath,
        fetch: F,
    ) -> Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, objectiveai_sdk::error::ResponseError>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, objectiveai_sdk::error::ResponseError>> + Send,
    {
        cached_get_or_fetch(&self.agent_cache, key, fetch).await
    }

    pub async fn cached_swarm<F, Fut>(
        &self,
        key: objectiveai_sdk::RemotePath,
        fetch: F,
    ) -> Result<Option<objectiveai_sdk::swarm::RemoteSwarmBase>, objectiveai_sdk::error::ResponseError>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Option<objectiveai_sdk::swarm::RemoteSwarmBase>, objectiveai_sdk::error::ResponseError>> + Send,
    {
        cached_get_or_fetch(&self.swarm_cache, key, fetch).await
    }

    pub async fn cached_function<F, Fut>(
        &self,
        key: objectiveai_sdk::RemotePath,
        fetch: F,
    ) -> Result<Option<objectiveai_sdk::functions::FullRemoteFunction>, objectiveai_sdk::error::ResponseError>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Option<objectiveai_sdk::functions::FullRemoteFunction>, objectiveai_sdk::error::ResponseError>> + Send,
    {
        cached_get_or_fetch(&self.function_cache, key, fetch).await
    }

    pub async fn cached_profile<F, Fut>(
        &self,
        key: objectiveai_sdk::RemotePath,
        fetch: F,
    ) -> Result<Option<objectiveai_sdk::functions::RemoteProfile>, objectiveai_sdk::error::ResponseError>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Option<objectiveai_sdk::functions::RemoteProfile>, objectiveai_sdk::error::ResponseError>> + Send,
    {
        cached_get_or_fetch(&self.profile_cache, key, fetch).await
    }

    pub async fn cached_remote_latest<F, Fut>(
        &self,
        key: objectiveai_sdk::RemotePathCommitOptional,
        fetch: F,
    ) -> Result<Option<objectiveai_sdk::RemotePath>, objectiveai_sdk::error::ResponseError>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Option<objectiveai_sdk::RemotePath>, objectiveai_sdk::error::ResponseError>> + Send,
    {
        cached_get_or_fetch(&self.remote_latest_cache, key, fetch).await
    }
}

impl<CTXEXT: super::ContextExt> Context<CTXEXT> {
    /// Returns the resolved upstream BYOK API key.
    ///
    /// Only OpenRouter is supported. Returns `None` for other upstreams.
    /// Checks the per-request token first, falls back to the BYOK token
    /// from the context extension. Result is cached for subsequent calls.
    pub async fn upstream_authorization(
        &self,
        upstream: objectiveai_sdk::agent::Upstream,
    ) -> Option<Arc<String>> {
        if upstream != objectiveai_sdk::agent::Upstream::Openrouter {
            return None;
        }
        self.openrouter_authorization_cached
            .get_or_init(|| async {
                match (&self.openrouter_authorization, self.ext.openrouter_authorization().await) {
                    (Some(self_token), _) => Some(self_token.clone()),
                    (None, byok) => byok,
                }
            })
            .await
            .clone()
    }

    /// Returns the resolved GitHub authorization token.
    ///
    /// Checks the per-request token first, falls back to the BYOK token
    /// from the context extension. Result is cached for subsequent calls.
    pub async fn github_authorization(&self) -> Option<Arc<String>> {
        self.github_authorization_cached
            .get_or_init(|| async {
                match (&self.github_authorization, self.ext.github_authorization().await) {
                    (Some(self_token), _) => Some(self_token.clone()),
                    (None, byok) => byok,
                }
            })
            .await
            .clone()
    }

    /// Returns the resolved MCP authorization headers.
    ///
    /// Merges the per-request headers with BYOK headers from the context
    /// extension. Per-request headers take priority over BYOK headers.
    /// Result is cached for subsequent calls.
    pub async fn mcp_authorization(&self) -> Option<Arc<HashMap<String, String>>> {
        self.mcp_authorization_cached
            .get_or_init(|| async {
                let byok: Option<Arc<HashMap<String, String>>> = self.ext.mcp_authorization().await;
                match (&self.mcp_authorization, byok) {
                    (None, None) => None,
                    (Some(self_headers), None) => Some(self_headers.clone()),
                    (None, Some(byok_headers)) => Some(byok_headers),
                    (Some(self_headers), Some(byok_headers)) => {
                        let mut merged: HashMap<String, String> = (**self_headers).clone();
                        for (k, v) in byok_headers.iter() {
                            merged.insert(k.clone(), v.clone());
                        }
                        Some(Arc::new(merged))
                    }
                }
            })
            .await
            .clone()
    }

}
