//! Request context containing per-request state and caches.

use dashmap::DashMap;
use futures::future::Shared;
use futures::FutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::OnceCell;
use super::persistent_cache::PersistentCacheClient;

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
pub struct Context<CTXEXT, PC> {
    /// Custom context extension (e.g., for BYOK keys).
    pub ext: Arc<CTXEXT>,
    /// Multiplier applied to costs for this request.
    pub cost_multiplier: rust_decimal::Decimal,
    /// Whether to suppress output (eprintln, logging, etc).
    pub suppress_output: bool,
    /// Persistent cache client for key-value storage.
    persistent_cache: Arc<PC>,
    /// Per-request ObjectiveAI authorization token.
    objectiveai_authorization: Option<Arc<String>>,
    /// Per-request OpenRouter authorization token.
    openrouter_authorization: Option<Arc<String>>,
    /// Per-request GitHub authorization token.
    github_authorization: Option<Arc<String>>,
    /// Per-request MCP authorization headers.
    mcp_authorization: Option<Arc<HashMap<String, String>>>,
    /// Per-request viewer signature.
    viewer_signature: Option<Arc<String>>,
    /// Per-request viewer address.
    viewer_address: Option<Arc<String>>,
    /// Per-request commit author name.
    commit_author_name: Option<Arc<String>>,
    /// Per-request commit author email.
    commit_author_email: Option<Arc<String>>,
    /// Per-request caller-supplied agent id (`X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`).
    /// Plays the role of the *parent* when composing the agent id we
    /// forward to the MCP proxy inside agent completions.
    agent_instance_hierarchy: Option<Arc<String>>,
    /// Loopback-only MCP listener port the API process bound. Used
    /// to synthesize
    /// `http://127.0.0.1:{mcp_port}/objectiveai-mcp/{ws_session_id}`
    /// reverse-attach URLs when an agent declares `client_objectiveai_mcp`.
    /// `None` on HTTP/SSE requests (no reverse-attach possible) and
    /// when running outside a bound server.
    mcp_port: Option<u16>,
    /// Handle for registering per-agent `ws_session_id`s against the
    /// current WS reverse channel. Set on WS-attached requests by the
    /// streaming handlers; `None` on HTTP/SSE. Many ids may register
    /// against the same handle — one per swarm agent that declares
    /// `client_objectiveai_mcp` — all cleaned up when the owning
    /// [`crate::streaming_ws::ReverseAttachGuard`] drops at WS close.
    reverse_attach: Option<Arc<crate::streaming_ws::ReverseAttachHandle>>,
    /// Cached resolved OpenRouter authorization (self + ext).
    openrouter_authorization_cached: Arc<OnceCell<Option<Arc<String>>>>,
    /// Cached resolved GitHub authorization (self + ext).
    github_authorization_cached: Arc<OnceCell<Option<Arc<String>>>>,
    /// Cached resolved MCP authorization (self + ext merged).
    mcp_authorization_cached: Arc<OnceCell<Option<Arc<HashMap<String, String>>>>>,
    /// Cached resolved viewer signature (self + ext).
    viewer_signature_cached: Arc<OnceCell<Option<Arc<String>>>>,
    /// Cached resolved viewer address (self + ext).
    viewer_address_cached: Arc<OnceCell<Option<Arc<String>>>>,
    /// Cached resolved commit author name (self + ext).
    commit_author_name_cached: Arc<OnceCell<Option<Arc<String>>>>,
    /// Cached resolved commit author email (self + ext).
    commit_author_email_cached: Arc<OnceCell<Option<Arc<String>>>>,
    /// Cancellation signal — set to true when the client disconnects.
    cancelled: Arc<std::sync::atomic::AtomicBool>,
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

impl<CTXEXT, PC> Clone for Context<CTXEXT, PC> {
    fn clone(&self) -> Self {
        Self {
            ext: self.ext.clone(),
            cost_multiplier: self.cost_multiplier,
            suppress_output: self.suppress_output,
            persistent_cache: self.persistent_cache.clone(),
            objectiveai_authorization: self.objectiveai_authorization.clone(),
            openrouter_authorization: self.openrouter_authorization.clone(),
            github_authorization: self.github_authorization.clone(),
            mcp_authorization: self.mcp_authorization.clone(),
            viewer_signature: self.viewer_signature.clone(),
            viewer_address: self.viewer_address.clone(),
            commit_author_name: self.commit_author_name.clone(),
            commit_author_email: self.commit_author_email.clone(),
            agent_instance_hierarchy: self.agent_instance_hierarchy.clone(),
            mcp_port: self.mcp_port,
            reverse_attach: self.reverse_attach.clone(),
            openrouter_authorization_cached: self.openrouter_authorization_cached.clone(),
            github_authorization_cached: self.github_authorization_cached.clone(),
            mcp_authorization_cached: self.mcp_authorization_cached.clone(),
            viewer_signature_cached: self.viewer_signature_cached.clone(),
            viewer_address_cached: self.viewer_address_cached.clone(),
            commit_author_name_cached: self.commit_author_name_cached.clone(),
            commit_author_email_cached: self.commit_author_email_cached.clone(),
            cancelled: self.cancelled.clone(),
            swarm_cache: self.swarm_cache.clone(),
            agent_cache: self.agent_cache.clone(),
            function_cache: self.function_cache.clone(),
            profile_cache: self.profile_cache.clone(),
            remote_latest_cache: self.remote_latest_cache.clone(),
        }
    }
}

impl<CTXEXT, PC> Context<CTXEXT, PC> {
    /// Returns whether this context has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Marks this context as cancelled.
    pub fn cancel(&self) {
        self.cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
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
        persistent_cache: Arc<PC>,
        cost_multiplier: rust_decimal::Decimal,
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

        let viewer_signature = headers
            .get("X-VIEWER-SIGNATURE")
            .or_else(|| headers.get("VIEWER-SIGNATURE"))
            .or_else(|| headers.get("X-OBJECTIVEAI-SIGNATURE"))
            .or_else(|| headers.get("OBJECTIVEAI-SIGNATURE"))
            .and_then(|v| v.to_str().ok())
            .map(|s| Arc::new(s.to_owned()));

        let viewer_address = headers
            .get("X-VIEWER-ADDRESS")
            .or_else(|| headers.get("VIEWER-ADDRESS"))
            .and_then(|v| v.to_str().ok())
            .map(|s| Arc::new(s.to_owned()));

        let commit_author_name = headers
            .get("X-COMMIT-AUTHOR-NAME")
            .or_else(|| headers.get("COMMIT-AUTHOR-NAME"))
            .and_then(|v| v.to_str().ok())
            .map(|s| Arc::new(s.to_owned()));

        let commit_author_email = headers
            .get("X-COMMIT-AUTHOR-EMAIL")
            .or_else(|| headers.get("COMMIT-AUTHOR-EMAIL"))
            .and_then(|v| v.to_str().ok())
            .map(|s| Arc::new(s.to_owned()));

        let agent_instance_hierarchy = headers
            .get("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY")
            .or_else(|| headers.get("OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY"))
            .and_then(|v| v.to_str().ok())
            .map(|s| Arc::new(s.to_owned()));

        Self {
            ext,
            cost_multiplier,
            suppress_output,
            persistent_cache,
            openrouter_authorization,
            github_authorization,
            mcp_authorization,
            objectiveai_authorization,
            viewer_signature,
            viewer_address,
            commit_author_name,
            commit_author_email,
            agent_instance_hierarchy,
            mcp_port: None,
            reverse_attach: None,
            openrouter_authorization_cached: Arc::new(OnceCell::new()),
            github_authorization_cached: Arc::new(OnceCell::new()),
            mcp_authorization_cached: Arc::new(OnceCell::new()),
            viewer_signature_cached: Arc::new(OnceCell::new()),
            viewer_address_cached: Arc::new(OnceCell::new()),
            commit_author_name_cached: Arc::new(OnceCell::new()),
            commit_author_email_cached: Arc::new(OnceCell::new()),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            swarm_cache: Arc::new(DashMap::new()),
            agent_cache: Arc::new(DashMap::new()),
            function_cache: Arc::new(DashMap::new()),
            profile_cache: Arc::new(DashMap::new()),
            remote_latest_cache: Arc::new(DashMap::new()),
        }
    }
}

impl<CTXEXT, PC> Context<CTXEXT, PC> {
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

    /// Returns the loopback-only MCP listener port the API process
    /// bound, if a streaming WS handler stamped one on this context.
    pub fn mcp_port(&self) -> Option<u16> {
        self.mcp_port
    }

    /// Returns the shared reverse-attach handle for registering
    /// per-agent `ws_session_id`s against the current WS, if a
    /// streaming WS handler stamped one.
    pub fn reverse_attach(
        &self,
    ) -> Option<&Arc<crate::streaming_ws::ReverseAttachHandle>> {
        self.reverse_attach.as_ref()
    }

    /// Stamps the loopback MCP listener port (from
    /// `ReverseAttachConfig.mcp_port`) on the context. Returns the
    /// modified context for chaining.
    pub fn with_mcp_port(mut self, port: u16) -> Self {
        self.mcp_port = Some(port);
        self
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
}

/// Check the in-memory DashMap, then the persistent cache, then call `fetch`.
/// Non-None results from `fetch` are written to the persistent cache.
/// All results (including None/Err) are cached in the DashMap for per-request dedup.
async fn cached_get_or_fetch<K, V, PC, F, Fut>(
    cache: &DashMap<
        K,
        Shared<tokio::sync::oneshot::Receiver<Result<Option<V>, objectiveai_sdk::error::ResponseError>>>,
    >,
    persistent_cache: &Arc<PC>,
    namespace: &'static str,
    key: K,
    permanent: bool,
    fetch: F,
) -> Result<Option<V>, objectiveai_sdk::error::ResponseError>
where
    K: std::hash::Hash + Eq + serde::Serialize + Clone + Send + Sync + 'static,
    V: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync + 'static,
    PC: PersistentCacheClient,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Option<V>, objectiveai_sdk::error::ResponseError>> + Send,
{
    let persistent_cache = persistent_cache.clone();
    let persistent_key = serde_json::to_string(&key).unwrap();
    let shared = cache
        .entry(key)
        .or_insert_with(|| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::spawn(async move {
                let from_persistent = persistent_cache
                    .get(namespace, &persistent_key)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|s| serde_json::from_str::<V>(&s).ok());

                if let Some(value) = from_persistent {
                    let _ = tx.send(Ok(Some(value)));
                } else {
                    let result = fetch().await;
                    // Serialize before sending so we can write to persistent cache after.
                    let json_to_persist = match &result {
                        Ok(Some(value)) => serde_json::to_string(value).ok(),
                        _ => None,
                    };
                    let _ = tx.send(result);
                    // Write to persistent cache after unblocking the caller.
                    if let Some(json) = json_to_persist {
                        let _ = persistent_cache.set(namespace, &persistent_key, &json, permanent).await;
                    }
                }
            });
            rx.shared()
        })
        .clone();
    shared.await.unwrap()
}

impl<CTXEXT, PC: PersistentCacheClient> Context<CTXEXT, PC> {
    pub async fn cached_agent<F, Fut>(
        &self,
        key: objectiveai_sdk::RemotePath,
        fetch: F,
    ) -> Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, objectiveai_sdk::error::ResponseError>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, objectiveai_sdk::error::ResponseError>> + Send,
    {
        cached_get_or_fetch(&self.agent_cache, &self.persistent_cache, "agent", key, true, fetch).await
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
        cached_get_or_fetch(&self.swarm_cache, &self.persistent_cache, "swarm", key, true, fetch).await
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
        cached_get_or_fetch(&self.function_cache, &self.persistent_cache, "function", key, true, fetch).await
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
        cached_get_or_fetch(&self.profile_cache, &self.persistent_cache, "profile", key, true, fetch).await
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
        cached_get_or_fetch(&self.remote_latest_cache, &self.persistent_cache, "remote_latest", key, false, fetch).await
    }
}

impl<CTXEXT: super::ContextExt, PC> Context<CTXEXT, PC> {
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

    /// Returns the resolved ObjectiveAI signature.
    ///
    /// Checks the per-request signature first, falls back to the BYOK signature
    /// from the context extension. Result is cached for subsequent calls.
    pub async fn viewer_signature(&self) -> Option<Arc<String>> {
        self.viewer_signature_cached
            .get_or_init(|| async {
                match (&self.viewer_signature, self.ext.viewer_signature().await) {
                    (Some(self_sig), _) => Some(self_sig.clone()),
                    (None, byok) => byok,
                }
            })
            .await
            .clone()
    }

    /// Returns the resolved ObjectiveAI viewer address.
    ///
    /// Checks the per-request address first, falls back to the BYOK address
    /// from the context extension. Result is cached for subsequent calls.
    pub async fn viewer_address(&self) -> Option<Arc<String>> {
        self.viewer_address_cached
            .get_or_init(|| async {
                match (&self.viewer_address, self.ext.viewer_address().await) {
                    (Some(self_addr), _) => Some(self_addr.clone()),
                    (None, byok) => byok,
                }
            })
            .await
            .clone()
    }

    /// Returns the resolved commit author name.
    ///
    /// Checks the per-request name first, falls back to the ext.
    /// Result is cached for subsequent calls.
    pub async fn commit_author_name(&self) -> Option<Arc<String>> {
        self.commit_author_name_cached
            .get_or_init(|| async {
                match (&self.commit_author_name, self.ext.commit_author_name().await) {
                    (Some(self_name), _) => Some(self_name.clone()),
                    (None, ext) => ext,
                }
            })
            .await
            .clone()
    }

    /// Returns the resolved commit author email.
    ///
    /// Checks the per-request email first, falls back to the ext.
    /// Result is cached for subsequent calls.
    pub async fn commit_author_email(&self) -> Option<Arc<String>> {
        self.commit_author_email_cached
            .get_or_init(|| async {
                match (&self.commit_author_email, self.ext.commit_author_email().await) {
                    (Some(self_email), _) => Some(self_email.clone()),
                    (None, ext) => ext,
                }
            })
            .await
            .clone()
    }
}
