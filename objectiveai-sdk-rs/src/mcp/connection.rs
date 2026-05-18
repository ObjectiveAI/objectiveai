//! MCP connection for communicating with an MCP server.
//!
//! [`Connection`] is a cheaply-clonable handle around an internal
//! [`ConnectionInner`]. The last drop of the inner `Arc` runs
//! [`ConnectionInner`]'s `Drop`, which cancels the listener task's
//! [`tokio_util::sync::CancellationToken`] (held in `_listener_cancel_guard`
//! as a [`tokio_util::sync::DropGuard`]) — the SSE listener exits the
//! instant any in-flight reconnect, sleep, or read is cancelled, with no
//! zombie 401 retries against a now-dead proxy session.

use std::ops::Deref;
use std::sync::{Arc, RwLock as StdRwLock, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use indexmap::IndexMap;
use tokio::sync::{Notify, RwLock};
use tokio_util::sync::{CancellationToken, DropGuard};

/// Callback fired by [`Connection`] when the upstream MCP server emits
/// `notifications/tools/list_changed` or `notifications/resources/list_changed`.
///
/// **Timing:** runs after the corresponding cache's write lock is taken
/// but *before* the network paginate that replaces it. That ordering
/// matches the moment the staleness window opens — anyone blocked on the
/// read lock won't return until the new list lands. The callback should
/// not call back into `list_tools` / `list_resources`: doing so would
/// re-take the lock the listener already holds and deadlock.
///
/// Stored behind an `Arc` so the listener task can cheaply clone it out
/// of the lock and call it without holding the read guard.
pub type ListChangedCallback = Arc<dyn Fn() + Send + Sync + 'static>;

/// A registered-or-not callback slot. Wrapper so [`ConnectionInner`] can
/// keep `#[derive(Debug)]` (a raw `dyn Fn` isn't `Debug`).
struct CallbackSlot(StdRwLock<Option<ListChangedCallback>>);

impl CallbackSlot {
    fn new() -> Self {
        Self(StdRwLock::new(None))
    }

    fn set(&self, callback: ListChangedCallback) {
        *self.0.write().unwrap() = Some(callback);
    }

    /// Cheap clone-out of the current callback (if any). The `Arc` clone
    /// lets us release the read guard before invoking the callback.
    fn get(&self) -> Option<ListChangedCallback> {
        self.0.read().unwrap().clone()
    }
}

impl std::fmt::Debug for CallbackSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let set = self.0.read().map(|g| g.is_some()).unwrap_or(false);
        f.debug_struct("CallbackSlot").field("set", &set).finish()
    }
}

/// An active connection to an MCP server using the Streamable HTTP transport.
///
/// Cheaply clonable (one `Arc` bump). When the last clone is dropped, the
/// inner `Arc` ref count hits zero, [`ConnectionInner::Drop`] runs, the
/// listener-cancel `DropGuard` is dropped, and the SSE listener task is
/// cancelled — exiting any in-flight `lines.next_line()`, reconnect
/// `send()`, or backoff `sleep` *immediately* without retrying against
/// the now-dead proxy session.
///
/// Use the public methods (`list_tools`, `call_tool`, `list_resources`,
/// `read_resource`, `call_tool_as_message`, `tool_key`) for the upstream
/// MCP protocol surface. The inner state ([`ConnectionInner`]) is also
/// reachable via `Deref` for read-only field access (e.g.
/// `connection.url`, `connection.initialize_result.server_info.name`),
/// but its methods are private — you must go through `Connection`.
#[derive(Debug)]
pub struct Connection {
    inner: Arc<ConnectionInner>,
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

// No `Drop` for `Connection`: cancellation happens deterministically
// when the last `Arc<ConnectionInner>` clone is dropped, which runs
// `ConnectionInner::drop` and releases the cancel-token DropGuard.

impl Deref for Connection {
    type Target = ConnectionInner;
    fn deref(&self) -> &ConnectionInner {
        &self.inner
    }
}

impl Connection {
    pub(super) async fn new(
        http_client: reqwest::Client,
        url: String,
        session_id: String,
        headers: IndexMap<String, String>,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
        call_timeout: Duration,
        initialize_result: super::initialize_result::InitializeResult,
        initial_sse_lines: Option<super::LinesStream>,
    ) -> Self {
        let inner = ConnectionInner::new(
            http_client,
            url,
            session_id,
            headers,
            backoff_current_interval,
            backoff_initial_interval,
            backoff_randomization_factor,
            backoff_multiplier,
            backoff_max_interval,
            backoff_max_elapsed_time,
            call_timeout,
            initialize_result,
            initial_sse_lines,
        )
        .await;
        Self { inner }
    }


    pub(super) fn new_mock(url: String) -> Self {
        Self { inner: ConnectionInner::new_mock(url) }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(name: String, url: String) -> Self {
        Self { inner: ConnectionInner::new_for_test(name, url) }
    }

    /// Send a JSON-RPC notification to the upstream. Used by `Client`
    /// right after `initialize` to send `notifications/initialized`.
    pub(super) async fn notify<P: serde::Serialize>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<(), super::Error> {
        self.inner.notify(method, params).await
    }

    /// Returns a key identifying this connection for tool namespacing.
    pub fn tool_key(&self) -> String {
        self.inner.tool_key()
    }

    /// Returns the session ID for this connection.
    pub fn session_id(&self) -> &str {
        self.inner.session_id()
    }

    /// Returns all tools from the upstream server.
    pub async fn list_tools(
        &self,
    ) -> Result<Arc<Vec<super::tool::Tool>>, Arc<super::Error>> {
        self.inner.list_tools().await
    }

    /// Calls a tool on the upstream server.
    pub async fn call_tool(
        &self,
        params: &super::tool::CallToolRequestParams,
    ) -> Result<super::tool::CallToolResult, super::Error> {
        self.inner.call_tool(params).await
    }

    /// Calls a tool and converts the result into a [`ToolMessage`].
    pub async fn call_tool_as_message(
        &self,
        params: &super::tool::CallToolRequestParams,
        tool_call_id: String,
    ) -> Result<
        crate::agent::completions::message::ToolMessage,
        super::Error,
    > {
        self.inner.call_tool_as_message(params, tool_call_id).await
    }

    /// Returns all resources from the upstream server.
    pub async fn list_resources(
        &self,
    ) -> Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>> {
        self.inner.list_resources().await
    }

    /// Returns the cached tool list as soon as it differs from `current`,
    /// or waits up to `timeout` for the next `notifications/tools/list_changed`
    /// from the upstream server before re-reading.
    ///
    /// Wakes the moment a refresh writer takes the cache write lock, so
    /// the post-wake `read` is guaranteed to observe the new list rather
    /// than racing against the install. Safe to call from any number of
    /// tasks concurrently.
    pub async fn subscribe_tools(
        &self,
        current: &[super::tool::Tool],
        timeout: Duration,
    ) -> Result<Arc<Vec<super::tool::Tool>>, Arc<super::Error>> {
        self.inner.subscribe_tools(current, timeout).await
    }

    /// Resource counterpart of [`Connection::subscribe_tools`].
    pub async fn subscribe_resources(
        &self,
        current: &[super::resource::Resource],
        timeout: Duration,
    ) -> Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>> {
        self.inner.subscribe_resources(current, timeout).await
    }

    /// Atomically drain the proxy's `pending_notifications` queue for
    /// this session via `GET /notify` and return the queued content
    /// blocks. A second call returns `[]` until the next out-of-band
    /// `POST /notify`.
    ///
    /// Intended for use at the start of an agent turn so notifications
    /// queued between turns — when the prior turn ended without a tool
    /// call, or the user is starting a fresh continuation — surface as
    /// a user message instead of being lost. The proxy's existing
    /// `tools/call` response path still drains in-flight notifications
    /// arriving *during* a turn; this method covers the gap between
    /// turns.
    ///
    /// A 404 from the proxy (session unknown — possible after a proxy
    /// restart) is mapped to an empty `Vec` so callers do not need to
    /// distinguish "no notifications" from "lost session" at the use
    /// site; the next upstream call will surface the lost-session
    /// condition through its own error path.
    pub async fn drain_notifications(
        &self,
    ) -> Result<Vec<super::tool::ContentBlock>, super::Error> {
        self.inner.drain_notifications().await
    }

    /// Reads a resource from the upstream server.
    pub async fn read_resource(
        &self,
        uri: &str,
    ) -> Result<super::resource::ReadResourceResult, super::Error> {
        self.inner.read_resource(uri).await
    }

    /// Register a callback to fire whenever the upstream emits
    /// `notifications/tools/list_changed`.
    ///
    /// **Timing:** the callback runs *after* the tool cache's write lock
    /// is acquired but *before* the network paginate that replaces it.
    /// That means readers blocked on the read lock won't return until the
    /// new list is in place, and the callback observes the moment the
    /// staleness window opens. The proxy uses this to emit its own
    /// `notifications/tools/list_changed` to downstream clients at the
    /// right instant.
    ///
    /// Replaces any previously-registered tools-list-changed callback.
    /// All clones of this `Connection` share the same callback slot.
    pub fn set_on_tools_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.inner.on_tools_list_changed.set(Arc::new(callback));
    }

    /// Register a callback to fire whenever the upstream emits
    /// `notifications/resources/list_changed`. Same timing contract as
    /// [`Connection::set_on_tools_list_changed`].
    ///
    /// Replaces any previously-registered resources-list-changed callback.
    /// All clones of this `Connection` share the same callback slot.
    pub fn set_on_resources_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.inner.on_resources_list_changed.set(Arc::new(callback));
    }
}

/// The actual connection state. Behind an `Arc` inside [`Connection`].
///
/// Fields are public for read-only access (callers reach them via
/// `Connection`'s `Deref`), but every method on this type is private —
/// the public surface lives on [`Connection`] and delegates through.
#[derive(Debug)]
pub struct ConnectionInner {
    pub http_client: reqwest::Client,
    pub url: String,
    pub session_id: String,
    /// All HTTP headers stamped on every POST / GET this connection
    /// makes — the same merged map (defaults + caller overrides) the
    /// `Client` built once during connect. `Mcp-Session-Id`,
    /// `Content-Type`, and `Accept` are still set by the request
    /// builders and override anything in `headers`.
    pub headers: IndexMap<String, String>,

    pub backoff_current_interval: Duration,
    pub backoff_initial_interval: Duration,
    pub backoff_randomization_factor: f64,
    pub backoff_multiplier: f64,
    pub backoff_max_interval: Duration,
    pub backoff_max_elapsed_time: Duration,
    pub call_timeout: Duration,

    /// The server's capabilities and info from the initialize response.
    pub initialize_result: super::initialize_result::InitializeResult,

    /// If true, all RPC/notify calls are no-ops. Used for mock orchestrator URLs.
    mock: bool,

    /// Auto-incrementing request ID (starts at 2; 1 was used for initialize).
    next_id: AtomicU64,

    /// All tools from the server, populated by background pagination.
    tools: RwLock<Result<Arc<Vec<super::tool::Tool>>, Arc<super::Error>>>,
    /// All resources from the server, populated by background pagination.
    resources:
        RwLock<Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>>>,

    /// Cancellation token for the long-lived `listen_for_list_changes`
    /// task. The listener selects this against every blocking await
    /// (read, reconnect-send, backoff-sleep) and returns the instant it
    /// fires.
    ///
    /// Held inside the connection as a [`DropGuard`] so that the moment
    /// the last `Arc<ConnectionInner>` clone is dropped — i.e. the
    /// moment no external `Connection` handle remains — `Drop` runs on
    /// the guard, the token cancels, and the listener task tears down.
    /// The listener itself holds a sibling `CancellationToken` (clone),
    /// not the guard, so its task does not extend the connection's
    /// lifetime.
    _listener_cancel_guard: Option<DropGuard>,

    /// Optional callback fired *after* the listener has refreshed the
    /// tool cache in response to an upstream `notifications/tools/list_changed`.
    /// Set via [`Connection::set_on_tools_list_changed`].
    on_tools_list_changed: CallbackSlot,

    /// Optional callback fired *after* the listener has refreshed the
    /// resource cache in response to an upstream
    /// `notifications/resources/list_changed`.
    /// Set via [`Connection::set_on_resources_list_changed`].
    on_resources_list_changed: CallbackSlot,

    /// Wakes any task awaiting in [`Connection::subscribe_tools`]. Fired
    /// from inside `refresh_tools_signaling` the moment the writer
    /// acquires the cache write lock — *before* the new list is
    /// installed. A woken subscriber's next `read().await` blocks behind
    /// the writer's guard, so it always observes the post-swap state.
    tools_changed: Notify,

    /// Resource counterpart of [`Self::tools_changed`].
    resources_changed: Notify,
}

impl ConnectionInner {
    /// Creates a mock connection that never makes network requests.
    /// All RPC calls return empty/default results.
    fn new_mock(url: String) -> Arc<Self> {
        Arc::new(Self {
            http_client: reqwest::Client::new(),
            url,
            session_id: String::new(),
            headers: IndexMap::new(),
            backoff_current_interval: Duration::ZERO,
            backoff_initial_interval: Duration::ZERO,
            backoff_randomization_factor: 0.0,
            backoff_multiplier: 1.0,
            backoff_max_interval: Duration::ZERO,
            backoff_max_elapsed_time: Duration::ZERO,
            call_timeout: Duration::ZERO,
            initialize_result: super::initialize_result::InitializeResult {
                protocol_version: "2025-03-26".into(),
                capabilities: super::initialize_result::ServerCapabilities {
                    experimental: None,
                    logging: None,
                    completions: None,
                    prompts: None,
                    resources: None,
                    tools: None,
                    tasks: None,
                },
                server_info: super::initialize_result::Implementation {
                    name: "mock".into(),
                    title: None,
                    version: "0.0.0".into(),
                    website_url: None,
                    description: None,
                    icons: None,
                },
                instructions: None,
                _meta: None,
            },
            mock: true,
            next_id: AtomicU64::new(2),
            tools: RwLock::new(Ok(Arc::new(Vec::new()))),
            resources: RwLock::new(Ok(Arc::new(Vec::new()))),
            _listener_cancel_guard: None,
            on_tools_list_changed: CallbackSlot::new(),
            on_resources_list_changed: CallbackSlot::new(),
            tools_changed: Notify::new(),
            resources_changed: Notify::new(),
        })
    }

    /// Creates a minimal connection for unit testing.
    #[cfg(test)]
    fn new_for_test(name: String, url: String) -> Arc<Self> {
        Arc::new(Self {
            http_client: reqwest::Client::new(),
            url,
            session_id: String::new(),
            headers: IndexMap::new(),
            backoff_current_interval: Duration::from_millis(500),
            backoff_initial_interval: Duration::from_millis(500),
            backoff_randomization_factor: 0.5,
            backoff_multiplier: 1.5,
            backoff_max_interval: Duration::from_secs(60),
            backoff_max_elapsed_time: Duration::from_secs(900),
            call_timeout: Duration::from_secs(30),
            initialize_result: super::initialize_result::InitializeResult {
                protocol_version: "2025-03-26".into(),
                capabilities:
                    super::initialize_result::ServerCapabilities {
                        experimental: None,
                        logging: None,
                        completions: None,
                        prompts: None,
                        resources: None,
                        tools: None,
                        tasks: None,
                    },
                server_info: super::initialize_result::Implementation {
                    name,
                    title: None,
                    version: "0.0.0".into(),
                    website_url: None,
                    description: None,
                    icons: None,
                },
                instructions: None,
                _meta: None,
            },
            mock: false,
            next_id: AtomicU64::new(2),
            tools: RwLock::new(Ok(Arc::new(Vec::new()))),
            resources: RwLock::new(Ok(Arc::new(Vec::new()))),
            _listener_cancel_guard: None,
            on_tools_list_changed: CallbackSlot::new(),
            on_resources_list_changed: CallbackSlot::new(),
            tools_changed: Notify::new(),
            resources_changed: Notify::new(),
        })
    }

    /// Creates a new connection and spawns background tasks to paginate
    /// all tools and resources. Called internally by
    /// [`Client::connect`](super::Client::connect) (via [`Connection::new`]).
    ///
    /// `initial_sse_lines`, if `Some`, is a pre-opened SSE line reader
    /// that the list-changed listener will read from immediately on its
    /// first iteration, instead of opening its own GET `/`. The caller
    /// is responsible for arranging for one of these to exist whenever
    /// the upstream advertises `tools.list_changed` or
    /// `resources.list_changed` — see
    /// [`Client::connect`](super::Client::connect).
    async fn new(
        http_client: reqwest::Client,
        url: String,
        session_id: String,
        headers: IndexMap<String, String>,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
        call_timeout: Duration,
        initialize_result: super::initialize_result::InitializeResult,
        initial_sse_lines: Option<super::LinesStream>,
    ) -> Arc<Self> {
        // Cancel-the-listener machinery: store the DropGuard inside the
        // inner so the cancellation fires deterministically when the
        // last external `Arc<ConnectionInner>` clone drops. Hand the
        // listener task a sibling clone (no guard) — that way the
        // listener task's lifetime does not extend the connection.
        let listener_cancel = CancellationToken::new();
        let listener_cancel_for_task = listener_cancel.clone();
        let conn = Arc::new(Self {
            http_client,
            url,
            session_id,
            headers,
            backoff_current_interval,
            backoff_initial_interval,
            backoff_randomization_factor,
            backoff_multiplier,
            backoff_max_interval,
            backoff_max_elapsed_time,
            call_timeout,
            initialize_result,
            mock: false,
            next_id: AtomicU64::new(2),
            tools: RwLock::new(Ok(Arc::new(Vec::new()))),
            resources: RwLock::new(Ok(Arc::new(Vec::new()))),
            _listener_cancel_guard: Some(listener_cancel.drop_guard()),
            on_tools_list_changed: CallbackSlot::new(),
            on_resources_list_changed: CallbackSlot::new(),
            tools_changed: Notify::new(),
            resources_changed: Notify::new(),
        });

        // Spawn background tool lister if the server supports tools.
        //
        // We don't return until the spawned task has acquired the write
        // lock. Otherwise a caller that immediately reads `list_tools()`
        // could race the writer — `tokio::spawn` only queues the task,
        // and a fast reader can acquire the read lock before the writer
        // has run its first instruction. The reader would then see the
        // initial empty `Vec` and return that, even though a full
        // populate is in flight.
        //
        // The `RwLockWriteGuard` itself isn't `Send`-friendly enough to
        // pass back, so we use a oneshot to signal "I'm holding the
        // lock now"; once we receive that, the cache is exclusively
        // owned by the writer and any subsequent `read().await` from
        // the caller is guaranteed to wait for the populate to finish.
        if conn.initialize_result.capabilities.tools.is_some() {
            let conn = Arc::clone(&conn);
            let (lock_held_tx, lock_held_rx) = tokio::sync::oneshot::channel();
            tokio::spawn(async move {
                conn.refresh_tools_signaling(lock_held_tx, None).await;
            });
            // Wait for the writer to hold the lock before returning.
            let _ = lock_held_rx.await;
        }

        // Spawn background resource lister if the server supports
        // resources. Same lock-handoff contract as tools above.
        if conn.initialize_result.capabilities.resources.is_some() {
            let conn = Arc::clone(&conn);
            let (lock_held_tx, lock_held_rx) = tokio::sync::oneshot::channel();
            tokio::spawn(async move {
                conn.refresh_resources_signaling(lock_held_tx, None).await;
            });
            let _ = lock_held_rx.await;
        }

        // Spawn the list-changed listener iff the caller handed us a
        // pre-opened SSE stream. The connection is naive about
        // `tools.list_changed` / `resources.list_changed` capabilities —
        // [`Client::connect`](super::Client::connect) translates them
        // into "did or didn't open a stream for us." If we get a stream,
        // we listen on it; if we don't, there's nothing to listen for.
        if let Some(initial_lines) = initial_sse_lines {
            // Hand the listener a `Weak` so the spawned task itself does
            // not keep the connection alive. `listener_cancel_for_task`
            // is a sibling clone of the connection's own
            // `_listener_cancel_guard` token — when the last external
            // `Arc<ConnectionInner>` clone is dropped, the inner's Drop
            // releases the guard and the listener wakes from any
            // pending await (read, send, sleep) and exits immediately.
            let weak = Arc::downgrade(&conn);
            tokio::spawn(async move {
                Self::listen_for_list_changes(
                    weak,
                    listener_cancel_for_task,
                    initial_lines,
                )
                .await;
            });
        }

        conn
    }

    /// Creates an exponential backoff configuration from the connection's fields.
    fn backoff(&self) -> backoff::ExponentialBackoff {
        backoff::ExponentialBackoff {
            current_interval: self.backoff_current_interval,
            initial_interval: self.backoff_initial_interval,
            randomization_factor: self.backoff_randomization_factor,
            multiplier: self.backoff_multiplier,
            max_interval: self.backoff_max_interval,
            start_time: std::time::Instant::now(),
            max_elapsed_time: Some(self.backoff_max_elapsed_time),
            clock: backoff::SystemClock::default(),
        }
    }

    /// Builds a POST request with all required headers and the call timeout.
    fn post(&self) -> reqwest::RequestBuilder {
        let mut request = self
            .http_client
            .post(&self.url)
            .timeout(self.call_timeout)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }
        // Mcp-Session-Id is applied last so a same-named entry in
        // `headers` (e.g. the proxy's encoded session id) can never
        // override the connection's own session id.
        request = request.header("Mcp-Session-Id", &self.session_id);
        request
    }

    /// Sends a JSON-RPC request, retrying transient errors when
    /// `idempotent` is `true`.
    ///
    /// Idempotent methods (`tools/list`, `resources/list`,
    /// `resources/read`, etc.) retry every transient error — network,
    /// HTTP status, malformed body, JSON-RPC error, session expiration —
    /// until the backoff's `max_elapsed_time` is exceeded.
    ///
    /// Non-idempotent methods (`tools/call`) make exactly one attempt.
    /// Retrying a `tools/call` is unsafe: a tool may have mutated remote
    /// state during the first attempt before the response was lost, and
    /// re-firing the call would mutate state again. Each retry of
    /// `AppendTask` advances `state.tasks.len()` an extra step, so the
    /// agent sees a different return value than expected and the
    /// pid-derived mock seed at the next step diverges. See
    /// `objectiveai-api/src/agent/completions/client.rs` (sequential
    /// dispatch) and `mock/client.rs::mock.seed_derive` for the
    /// downstream consequence.
    async fn rpc<P: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &P,
        idempotent: bool,
    ) -> Result<R, super::Error> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let attempt_one = || async {
            let url = self.url.clone();
            let response = self.post().json(&body).send().await.map_err(|source| {
                backoff::Error::transient(super::Error::Request {
                    url: url.clone(),
                    source,
                })
            })?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(backoff::Error::transient(
                    super::Error::SessionExpired { url: url.clone() },
                ));
            }
            if !response.status().is_success() {
                let code = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(backoff::Error::transient(
                    super::Error::BadStatus { url: url.clone(), code, body },
                ));
            }

            let rpc_response: super::JsonRpcResponse<R> =
                super::parse_streamable_http_response(&url, response)
                    .await
                    .map_err(backoff::Error::transient)?;

            match rpc_response {
                super::JsonRpcResponse::Success { result, .. } => Ok(result),
                super::JsonRpcResponse::Error { error, .. } => {
                    Err(backoff::Error::transient(super::Error::JsonRpc {
                        url: url.clone(),
                        code: error.code,
                        message: error.message,
                        data: error.data,
                    }))
                }
            }
        };

        if idempotent {
            backoff::future::retry(self.backoff(), attempt_one).await
        } else {
            attempt_one().await.map_err(|e| match e {
                backoff::Error::Permanent(err) | backoff::Error::Transient { err, .. } => err,
            })
        }
    }

    /// Sends a JSON-RPC notification (no response expected) with the
    /// same exponential-backoff retry policy as [`Self::rpc`]. Every
    /// error is transient; the loop gives up only when the backoff's
    /// `max_elapsed_time` is exceeded.
    async fn notify<P: serde::Serialize>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<(), super::Error> {
        if self.mock { return Ok(()); }
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        backoff::future::retry(self.backoff(), || async {
            let url = self.url.clone();
            let response = self.post().json(&body).send().await.map_err(|source| {
                backoff::Error::transient(super::Error::Request {
                    url: url.clone(),
                    source,
                })
            })?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(backoff::Error::transient(
                    super::Error::SessionExpired { url: url.clone() },
                ));
            }
            if !response.status().is_success() {
                let code = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(backoff::Error::transient(
                    super::Error::BadStatus { url: url.clone(), code, body },
                ));
            }

            Ok(())
        })
        .await
    }

    /// `GET <self.url>/notify` against the ObjectiveAI MCP proxy.
    /// Atomically drains the proxy's pending-notifications queue for
    /// this session and returns the queued content blocks.
    ///
    /// Single-attempt — the proxy drain is destructive, so a retry
    /// after a transient failure would risk silently dropping
    /// notifications that the first attempt's response carried but
    /// failed to deliver. Networks errors propagate to the caller; the
    /// next turn's drain will pick up anything queued in the meantime.
    /// A 404 (session unknown) is mapped to `Ok(vec![])` — see the
    /// public method's doc on `Connection`.
    async fn drain_notifications(
        &self,
    ) -> Result<Vec<super::tool::ContentBlock>, super::Error> {
        if self.mock {
            return Ok(Vec::new());
        }

        let url = format!("{}/notify", self.url.trim_end_matches('/'));
        let mut request = self
            .http_client
            .get(&url)
            .timeout(self.call_timeout)
            .header("Accept", "application/json");
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }
        // Mcp-Session-Id applied last so a same-named entry in `headers`
        // can never override the connection's own session id — matches
        // the invariant in `Self::post`.
        request = request.header("Mcp-Session-Id", &self.session_id);

        let response = request.send().await.map_err(|source| super::Error::Request {
            url: url.clone(),
            source,
        })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        if !response.status().is_success() {
            let code = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(super::Error::BadStatus { url, code, body });
        }

        response
            .json::<Vec<super::tool::ContentBlock>>()
            .await
            .map_err(|source| super::Error::Request { url, source })
    }

    /// Returns a key identifying this connection for tool namespacing.
    fn tool_key(&self) -> String {
        format!("{}-{}", self.initialize_result.server_info.name, self.url)
    }

    /// Returns the session ID for this connection.
    fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Sends a `tools/list` RPC call for a single page.
    async fn rpc_list_tools(
        &self,
        cursor: Option<&str>,
    ) -> Result<super::tool::ListToolsResult, super::Error> {
        self.rpc(
            "tools/list",
            &super::tool::ListToolsRequest {
                cursor: cursor.map(String::from),
            },
            true,
        )
        .await
    }

    /// Returns all tools from the server.
    ///
    /// Blocks until background pagination completes, then returns a
    /// cheap `Arc` clone of the result.
    async fn list_tools(
        &self,
    ) -> Result<Arc<Vec<super::tool::Tool>>, Arc<super::Error>> {
        self.tools.read().await.clone()
    }

    /// Calls a tool on the MCP server.
    async fn call_tool(
        &self,
        params: &super::tool::CallToolRequestParams,
    ) -> Result<super::tool::CallToolResult, super::Error> {
        if self.mock {
            return Ok(super::tool::CallToolResult {
                content: vec![super::tool::ContentBlock::Text(super::tool::TextContent {
                    text: "mock".to_string(),
                    annotations: None,
                    _meta: None,
                })],
                structured_content: None,
                is_error: None,
                _meta: None,
            });
        }
        self.rpc("tools/call", params, false).await
    }

    /// Calls a tool and converts the result into a [`ToolMessage`].
    ///
    /// Content blocks are mapped as follows:
    /// - `text` → text part
    /// - `image` → image_url part (data URL)
    /// - `audio` → input_audio part
    /// - `resource` (embedded text) → text part
    /// - `resource` (embedded blob, image mime) → image_url part (data URL)
    /// - `resource` (embedded blob, other mime) → file part
    /// - `resource_link` → if the URI appears in `list_resources`, fetches
    ///   via `read_resource` and inlines the content using the same
    ///   text/blob rules; otherwise serializes the link as JSON text
    ///
    /// If `is_error` is set on the result, the content is prefixed with
    /// an error indicator.
    async fn call_tool_as_message(
        &self,
        params: &super::tool::CallToolRequestParams,
        tool_call_id: String,
    ) -> Result<
        crate::agent::completions::message::ToolMessage,
        super::Error,
    > {
        use crate::agent::completions::message::{
            File, ImageUrl, InputAudio, RichContent, RichContentPart,
            ToolMessage,
        };
        use super::shared::ResourceContentsUnion;
        use super::tool::ContentBlock;

        let result = self.call_tool(params).await?;

        // Build the set of known resource URIs for resource_link resolution.
        let known_resource_uris: std::collections::HashSet<String> =
            match self.list_resources().await {
                Ok(resources) => {
                    resources.iter().map(|r| r.uri.clone()).collect()
                }
                Err(_) => std::collections::HashSet::new(),
            };

        /// Converts a `ResourceContentsUnion` into one or more rich content
        /// parts. Text resources become text parts. Blob resources with an
        /// image MIME type become image_url parts (data URL); all other blobs
        /// become file parts.
        fn resource_contents_to_part(
            contents: &ResourceContentsUnion,
        ) -> RichContentPart {
            match contents {
                ResourceContentsUnion::Text(text) => {
                    RichContentPart::Text {
                        text: text.text.clone(),
                    }
                }
                ResourceContentsUnion::Blob(blob) => {
                    let mime = blob
                        .base
                        .mime_type
                        .as_deref()
                        .unwrap_or("application/octet-stream");

                    if mime.starts_with("image/") {
                        RichContentPart::ImageUrl {
                            image_url: ImageUrl {
                                url: format!(
                                    "data:{};base64,{}",
                                    mime, blob.blob
                                ),
                                detail: None,
                            },
                        }
                    } else {
                        // Extract a filename from the URI path, if any.
                        let filename = blob
                            .base
                            .uri
                            .rsplit('/')
                            .next()
                            .filter(|s| !s.is_empty())
                            .map(String::from);

                        RichContentPart::File {
                            file: File {
                                file_data: Some(blob.blob.clone()),
                                filename,
                                file_id: None,
                                file_url: None,
                            },
                        }
                    }
                }
            }
        }

        let mut parts: Vec<RichContentPart> = Vec::new();

        for block in &result.content {
            match block {
                ContentBlock::Text(text) => {
                    parts.push(RichContentPart::Text {
                        text: text.text.clone(),
                    });
                }
                ContentBlock::Image(image) => {
                    parts.push(RichContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: format!(
                                "data:{};base64,{}",
                                image.mime_type, image.data
                            ),
                            detail: None,
                        },
                    });
                }
                ContentBlock::Audio(audio) => {
                    parts.push(RichContentPart::InputAudio {
                        input_audio: InputAudio {
                            data: audio.data.clone(),
                            format: audio.mime_type.clone(),
                        },
                    });
                }
                ContentBlock::EmbeddedResource(embedded) => {
                    parts.push(resource_contents_to_part(
                        &embedded.resource,
                    ));
                }
                ContentBlock::ResourceLink(link) => {
                    if known_resource_uris.contains(&link.uri) {
                        // Fetch the resource and inline its contents.
                        let read_result =
                            self.read_resource(&link.uri).await?;
                        for contents in &read_result.contents {
                            parts.push(
                                resource_contents_to_part(contents),
                            );
                        }
                    } else {
                        // Not a known resource; serialize as JSON text.
                        parts.push(RichContentPart::Text {
                            text: serde_json::to_string(link)
                                .unwrap_or_default(),
                        });
                    }
                }
            }
        }

        let content = match parts.len() {
            0 => RichContent::Text(String::new()),
            1 => match parts.remove(0) {
                RichContentPart::Text { text } => RichContent::Text(text),
                other => RichContent::Parts(vec![other]),
            },
            _ => RichContent::Parts(parts),
        };

        Ok(ToolMessage {
            content,
            tool_call_id,
        })
    }

    /// Sends a `resources/list` RPC call for a single page.
    async fn rpc_list_resources(
        &self,
        cursor: Option<&str>,
    ) -> Result<super::resource::ListResourcesResult, super::Error> {
        self.rpc(
            "resources/list",
            &super::resource::ListResourcesRequest {
                cursor: cursor.map(String::from),
            },
            true,
        )
        .await
    }

    /// Returns all resources from the server.
    ///
    /// Blocks until background pagination completes, then returns a
    /// cheap `Arc` clone of the result.
    async fn list_resources(
        &self,
    ) -> Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>> {
        self.resources.read().await.clone()
    }

    /// Returns the cached tool list as soon as it differs from `current`,
    /// or — if it equals `current` right now — waits up to `timeout` for
    /// the cache to change and then returns whatever it sees.
    ///
    /// An `Err` cache is treated as "different from any caller snapshot"
    /// and returned immediately.
    ///
    /// Concurrency-safe: any number of concurrent subscribers wait on
    /// independent `Notified` futures and read the cache through the
    /// shared `RwLock`. A timeout that fires alone is not an error — we
    /// re-read the cache and return whatever's there.
    async fn subscribe_tools(
        &self,
        current: &[super::tool::Tool],
        timeout: Duration,
    ) -> Result<Arc<Vec<super::tool::Tool>>, Arc<super::Error>> {
        // Arm BEFORE reading. `enable()` registers the future in the
        // wait queue without polling, so a `notify_waiters` racing
        // between our read and our await still wakes us.
        let notified = self.tools_changed.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();

        let initial = self.tools.read().await.clone();
        match &initial {
            Ok(arc) if arc.as_slice() == current => {}
            _ => return initial,
        }

        let _ = tokio::time::timeout(timeout, notified).await;

        self.tools.read().await.clone()
    }

    /// Resource counterpart of [`Self::subscribe_tools`].
    async fn subscribe_resources(
        &self,
        current: &[super::resource::Resource],
        timeout: Duration,
    ) -> Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>> {
        let notified = self.resources_changed.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();

        let initial = self.resources.read().await.clone();
        match &initial {
            Ok(arc) if arc.as_slice() == current => {}
            _ => return initial,
        }

        let _ = tokio::time::timeout(timeout, notified).await;

        self.resources.read().await.clone()
    }

    /// Reads a resource from the MCP server.
    async fn read_resource(
        &self,
        uri: &str,
    ) -> Result<super::resource::ReadResourceResult, super::Error> {
        self.rpc(
            "resources/read",
            &super::resource::ReadResourceRequestParams {
                uri: uri.to_string(),
            },
            true,
        )
        .await
    }

    /// Re-fetches all tools from the server, replacing the cached list.
    ///
    /// Optionally fires `on_change` *after* the write lock is acquired but
    /// *before* the network paginate begins, so the callback observes the
    /// "list change is in flight" edge — readers blocked on the read lock
    /// won't return until the new list lands. The proxy uses this to
    /// re-emit `notifications/tools/list_changed` to its downstream client
    /// at the moment the staleness window opens.
    async fn refresh_tools(&self, on_change: Option<ListChangedCallback>) {
        // Listener-driven refresh. Visibility contract: any caller
        // that issues `list_tools()` after a `tools/list_changed`
        // notification has been observed must see the post-swap
        // value, not stale data — so the write lock has to gate
        // readers across the upstream paginate.
        //
        // Performance contract: don't serialise paginate *behind*
        // lock-acquisition latency. We start `tools.write()` and the
        // upstream paginate **concurrently** with `tokio::join!`. The
        // write-lock acquire blocks new `list_tools()` readers
        // immediately (preserving visibility) and runs in parallel
        // with whatever drain time the in-flight readers need; the
        // paginate runs alongside. Total wall-clock is
        // `max(drain_time, paginate_time)` instead of the sum.
        //
        // `notify_waiters` and `on_change` fire under the write
        // guard, *after* `*guard = result`, so anyone awoken by them
        // queues on the read lock, waits for the guard to drop, and
        // observes the post-swap state.
        let (mut guard, result) = tokio::join!(
            self.tools.write(),
            self.paginate_tools(),
        );
        *guard = result;
        self.tools_changed.notify_waiters();
        if let Some(cb) = on_change {
            cb();
        }
    }

    /// Page-by-page fetch of the upstream tool list, no locks held.
    /// Shared between the `_signaling` (initial-populate, holds lock
    /// for the original "block fast readers" contract) and `refresh_*`
    /// (listener-driven, lock-only-around-install) variants.
    async fn paginate_tools(
        &self,
    ) -> Result<Arc<Vec<super::tool::Tool>>, Arc<super::Error>> {
        let mut all_tools = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            match self.rpc_list_tools(cursor.as_deref()).await {
                Ok(page) => {
                    all_tools.extend(page.tools);
                    cursor = page.next_cursor;
                    if cursor.is_none() {
                        return Ok(Arc::new(all_tools));
                    }
                }
                Err(e) => return Err(Arc::new(e)),
            }
        }
    }

    /// Same as [`Self::refresh_tools`] but fires `lock_held` once the
    /// write lock has been acquired so the caller can synchronise on
    /// "writer is in possession of the cache" before returning. Used by
    /// `ConnectionInner::new` to prevent a fast reader from acquiring
    /// the read lock before this writer has even started.
    async fn refresh_tools_signaling(
        &self,
        lock_held: tokio::sync::oneshot::Sender<()>,
        on_change: Option<ListChangedCallback>,
    ) {
        let mut guard = self.tools.write().await;
        // Fire `tools_changed` while we hold the write lock and *before*
        // installing the new list. Any subscriber woken now must take a
        // read lock to observe the result, and that read lock is queued
        // behind this write guard — so they always see the post-swap
        // state, never mid-swap.
        self.tools_changed.notify_waiters();
        let _ = lock_held.send(());
        if let Some(cb) = on_change {
            cb();
        }
        let mut all_tools = Vec::new();
        let mut cursor: Option<String> = None;
        let result = loop {
            match self.rpc_list_tools(cursor.as_deref()).await {
                Ok(page) => {
                    all_tools.extend(page.tools);
                    cursor = page.next_cursor;
                    if cursor.is_none() {
                        break Ok(Arc::new(all_tools));
                    }
                }
                Err(e) => break Err(Arc::new(e)),
            }
        };
        *guard = result;
    }

    /// Re-fetches all resources from the server, replacing the cached list.
    /// See [`ConnectionInner::refresh_tools`] for the callback timing
    /// contract.
    async fn refresh_resources(&self, on_change: Option<ListChangedCallback>) {
        // Same paginate-while-acquiring-the-write-lock pattern as
        // `refresh_tools` — see that comment for the visibility +
        // performance rationale.
        let (mut guard, result) = tokio::join!(
            self.resources.write(),
            self.paginate_resources(),
        );
        *guard = result;
        self.resources_changed.notify_waiters();
        if let Some(cb) = on_change {
            cb();
        }
    }

    async fn paginate_resources(
        &self,
    ) -> Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>> {
        let mut all_resources = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            match self.rpc_list_resources(cursor.as_deref()).await {
                Ok(page) => {
                    all_resources.extend(page.resources);
                    cursor = page.next_cursor;
                    if cursor.is_none() {
                        return Ok(Arc::new(all_resources));
                    }
                }
                Err(e) => return Err(Arc::new(e)),
            }
        }
    }

    /// Resource counterpart of [`Self::refresh_tools_signaling`].
    async fn refresh_resources_signaling(
        &self,
        lock_held: tokio::sync::oneshot::Sender<()>,
        on_change: Option<ListChangedCallback>,
    ) {
        let mut guard = self.resources.write().await;
        // See `refresh_tools_signaling` — fire under the write lock,
        // before install, so subscribers' next read sees the post-swap
        // state.
        self.resources_changed.notify_waiters();
        let _ = lock_held.send(());
        if let Some(cb) = on_change {
            cb();
        }
        let mut all_resources = Vec::new();
        let mut cursor: Option<String> = None;
        let result = loop {
            match self.rpc_list_resources(cursor.as_deref()).await {
                Ok(page) => {
                    all_resources.extend(page.resources);
                    cursor = page.next_cursor;
                    if cursor.is_none() {
                        break Ok(Arc::new(all_resources));
                    }
                }
                Err(e) => break Err(Arc::new(e)),
            }
        };
        *guard = result;
    }

    /// Builds a GET request to the MCP endpoint for receiving server
    /// notifications via SSE.
    fn get(&self) -> reqwest::RequestBuilder {
        let mut request = self
            .http_client
            .get(&self.url)
            .header("Accept", "text/event-stream");
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }
        // Mcp-Session-Id last so it always wins over `headers`.
        request = request.header("Mcp-Session-Id", &self.session_id);
        request
    }

    /// Listens for `notifications/tools/list_changed` and
    /// `notifications/resources/list_changed` on an SSE stream. On each
    /// notification, write-locks and re-fetches the full list.
    ///
    /// `initial_lines` is the pre-opened SSE line reader handed in by
    /// [`Client::connect`](super::Client::connect) — that stream is
    /// consumed first. When it ends (or any later GET reconnect ends),
    /// we sleep `backoff_initial_interval` and open a fresh GET `/` SSE
    /// stream.
    ///
    /// Takes a [`Weak<Self>`] (not `Arc<Self>`) so the spawned task
    /// doesn't itself keep the [`Connection`] alive, and a
    /// [`CancellationToken`] sibling clone of the connection's
    /// [`DropGuard`] so the task tears down the instant the last
    /// external `Arc<ConnectionInner>` clone is dropped — every
    /// blocking await (line read, reconnect send, backoff sleep) is
    /// raced against `cancel.cancelled()` and exits without any zombie
    /// retries against a now-dead session.
    async fn listen_for_list_changes(
        weak: Weak<Self>,
        cancel: CancellationToken,
        initial_lines: super::LinesStream,
    ) {
        // First iteration: use the pre-opened SSE stream the client
        // handed us. After that, fall back to opening fresh GET / SSE
        // streams as the upstream connection cycles.
        let mut next_lines: Option<super::LinesStream> = Some(initial_lines);
        // One-shot guard for the catch-up refresh: false on the very
        // first iteration (the caller's pre-opened SSE stream — its
        // associated cache was just populated by `Client::connect`'s
        // initial pagination, so re-fetching there would just be a
        // wasted round-trip), true thereafter. Every stream end —
        // whether `Ok(None)` (clean close) or `Err(_)` (read failure)
        // — drops back here, which we treat as an implicit
        // list-changed notification: the upstream's broadcast (in
        // particular the proxy's per-session `tokio::broadcast`) is
        // lossy for moments when this listener has zero active
        // subscribers, so anything that fired during our disconnect
        // window may have been dropped.
        //
        // ORDER MATTERS. The refresh must run AFTER we've re-opened
        // the GET / SSE stream — i.e. after we're a subscriber again
        // — and BEFORE we enter the inner read loop. If we refreshed
        // before the resubscribe, a notification that fired between
        // our refresh-completion and our subscribe would be lost the
        // same way as the original disconnect-window drops; doing it
        // after means a notification fired DURING the refresh lands
        // in the new subscriber's buffer (broadcast::Sender::send
        // backs onto each receiver's channel-capacity slot) and gets
        // consumed by the inner loop on its next read.
        let mut is_reconnect = false;

        loop {
            // The token cancels deterministically when the last
            // `Arc<ConnectionInner>` clone is dropped (see
            // `_listener_cancel_guard`). Check once per outer
            // iteration, but the real protection is the cancel arms in
            // every blocking await below — those exit immediately on
            // cancel.
            if cancel.is_cancelled() {
                return;
            }
            let Some(this) = weak.upgrade() else { return };
            let backoff_delay = this.backoff_initial_interval;

            let mut lines = match next_lines.take() {
                Some(l) => l,
                None => {
                    // Race the upstream GET against cancellation — if
                    // the connection drops mid-reconnect, exit
                    // immediately rather than waiting for the request
                    // to complete or time out (otherwise produces a
                    // burst of 401 retries against a now-dead session
                    // under heavy churn).
                    let send_outcome = tokio::select! {
                        out = this.get().send() => out,
                        _ = cancel.cancelled() => {
                            drop(this);
                            return;
                        }
                    };
                    let response = match send_outcome {
                        Ok(r) if r.status().is_success() => r,
                        _ => {
                            drop(this);
                            // Sleep with cancel-arm: instant exit on
                            // drop, no zombie retries.
                            tokio::select! {
                                _ = tokio::time::sleep(backoff_delay) => {}
                                _ = cancel.cancelled() => return,
                            }
                            continue;
                        }
                    };
                    super::lines_from_response(response)
                }
            };

            // Catch-up refresh on every reconnect — the implicit
            // list-changed treatment for the just-failed stream. See
            // the `is_reconnect` doc-comment above for the
            // refresh-AFTER-resubscribe rationale.
            if is_reconnect {
                // tools and resources are independent locks; run the
                // catch-up refreshes concurrently so disconnect
                // recovery isn't sequential.
                let _ = tokio::join!(
                    this.refresh_tools(this.on_tools_list_changed.get()),
                    this.refresh_resources(this.on_resources_list_changed.get()),
                );
            }
            is_reconnect = true;

            'inner: loop {
                tokio::select! {
                    line_result = lines.next_line() => {
                        match line_result {
                            Ok(Some(line)) => {
                                // SSE data lines start with "data: ".
                                let Some(data) = line.strip_prefix("data: ") else {
                                    continue 'inner;
                                };
                                let method = match serde_json::from_str::<super::JsonRpcNotification>(data) {
                                    Ok(n) => n.method,
                                    Err(_) => continue 'inner,
                                };
                                match method.as_str() {
                                    "notifications/tools/list_changed" => {
                                        // refresh_tools fires the
                                        // callback after the cache is
                                        // installed, so the proxy's
                                        // downstream
                                        // notifications/tools/list_changed
                                        // emission lines up with the
                                        // staleness window opening.
                                        this.refresh_tools(
                                            this.on_tools_list_changed.get(),
                                        )
                                        .await;
                                    }
                                    "notifications/resources/list_changed" => {
                                        this.refresh_resources(
                                            this.on_resources_list_changed.get(),
                                        )
                                        .await;
                                    }
                                    _ => {}
                                }
                            }
                            // Stream ended cleanly or errored — break out
                            // to the outer loop so we either reconnect or,
                            // if everyone's gone, exit at the top.
                            _ => break 'inner,
                        }
                    }
                    // Cancellation: the connection's last clone has
                    // dropped. Tear down immediately.
                    _ = cancel.cancelled() => {
                        drop(this);
                        return;
                    }
                }
            }

            // Stream ended — drop the strong ref before sleeping so the
            // next iteration's weak-upgrade can detect liveness honestly.
            drop(this);
            tokio::select! {
                _ = tokio::time::sleep(backoff_delay) => {}
                _ = cancel.cancelled() => return,
            }
        }
    }
}

#[cfg(test)]
mod subscribe_tests {
    use super::*;
    use crate::mcp::tool::{Tool, ToolSchemaObject, ToolSchemaType};

    fn tool(name: &str) -> Tool {
        Tool {
            name: name.to_string(),
            title: None,
            description: None,
            icons: None,
            input_schema: ToolSchemaObject {
                r#type: ToolSchemaType::Object,
                properties: None,
                required: None,
                extra: IndexMap::new(),
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        }
    }

    /// First read shows a different list — return immediately, never wait.
    #[tokio::test]
    async fn subscribe_tools_returns_immediately_when_cache_differs() {
        let conn = Connection::new_for_test("t".into(), "http://x".into());
        *conn.inner.tools.write().await = Ok(Arc::new(vec![tool("a")]));

        let start = std::time::Instant::now();
        let got = conn
            .subscribe_tools(&[tool("b")], Duration::from_secs(5))
            .await
            .unwrap();
        assert!(start.elapsed() < Duration::from_millis(100));
        assert_eq!(got.as_slice(), &[tool("a")]);
    }

    /// Cached `Err` is treated as "different from any caller snapshot."
    #[tokio::test]
    async fn subscribe_tools_returns_err_immediately() {
        let conn = Connection::new_for_test("t".into(), "http://x".into());
        let err = super::super::Error::NoSessionId {
            url: "http://x".into(),
            body: String::new(),
        };
        *conn.inner.tools.write().await = Err(Arc::new(err));

        let start = std::time::Instant::now();
        let got = conn
            .subscribe_tools(&[], Duration::from_secs(5))
            .await;
        assert!(start.elapsed() < Duration::from_millis(100));
        assert!(got.is_err());
    }

    /// Cache equals snapshot, then a writer fires the notify under the
    /// write lock and installs a new list. The subscriber wakes, then its
    /// re-read blocks behind the writer's guard, observes the new list.
    #[tokio::test]
    async fn subscribe_tools_wakes_on_change_and_reads_post_swap() {
        let conn = Connection::new_for_test("t".into(), "http://x".into());
        *conn.inner.tools.write().await = Ok(Arc::new(vec![tool("a")]));

        let conn_for_subscriber = conn.clone();
        let subscriber = tokio::spawn(async move {
            conn_for_subscriber
                .subscribe_tools(&[tool("a")], Duration::from_secs(5))
                .await
                .unwrap()
        });

        // Give the subscriber a moment to arm `notified()` and finish
        // its first read so it's parked on the timeout.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Simulate `refresh_tools_signaling`: take the write lock, fire
        // `tools_changed` *while holding* the write lock, then install
        // the new value before releasing. This is exactly the ordering
        // that the real refresh path uses.
        {
            let mut guard = conn.inner.tools.write().await;
            conn.inner.tools_changed.notify_waiters();
            // Hold briefly to make absolutely sure the subscriber is
            // racing the read lock against our drop.
            tokio::time::sleep(Duration::from_millis(20)).await;
            *guard = Ok(Arc::new(vec![tool("b")]));
        }

        let got = tokio::time::timeout(Duration::from_secs(2), subscriber)
            .await
            .expect("subscriber returned in time")
            .expect("subscriber didn't panic");
        assert_eq!(got.as_slice(), &[tool("b")]);
    }

    /// Cache equals snapshot, no notification arrives — timeout, return
    /// the still-equal list (not an error).
    #[tokio::test]
    async fn subscribe_tools_times_out_and_returns_unchanged_list() {
        let conn = Connection::new_for_test("t".into(), "http://x".into());
        *conn.inner.tools.write().await = Ok(Arc::new(vec![tool("a")]));

        let start = std::time::Instant::now();
        let got = conn
            .subscribe_tools(&[tool("a")], Duration::from_millis(50))
            .await
            .unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(40), "elapsed: {elapsed:?}");
        assert!(elapsed < Duration::from_millis(500), "elapsed: {elapsed:?}");
        assert_eq!(got.as_slice(), &[tool("a")]);
    }

    /// Two concurrent subscribers both wake on a single notify_waiters
    /// and both observe the post-swap list.
    #[tokio::test]
    async fn subscribe_tools_supports_concurrent_subscribers() {
        let conn = Connection::new_for_test("t".into(), "http://x".into());
        *conn.inner.tools.write().await = Ok(Arc::new(vec![tool("a")]));

        let c1 = conn.clone();
        let c2 = conn.clone();
        let s1 = tokio::spawn(async move {
            c1.subscribe_tools(&[tool("a")], Duration::from_secs(5))
                .await
                .unwrap()
        });
        let s2 = tokio::spawn(async move {
            c2.subscribe_tools(&[tool("a")], Duration::from_secs(5))
                .await
                .unwrap()
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        {
            let mut guard = conn.inner.tools.write().await;
            conn.inner.tools_changed.notify_waiters();
            *guard = Ok(Arc::new(vec![tool("c")]));
        }

        let (r1, r2) = tokio::join!(s1, s2);
        let r1 = r1.unwrap();
        let r2 = r2.unwrap();
        assert_eq!(r1.as_slice(), &[tool("c")]);
        assert_eq!(r2.as_slice(), &[tool("c")]);
    }
}

#[cfg(test)]
mod drain_notifications_tests {
    use super::*;
    use crate::mcp::tool::{ContentBlock, TextContent};
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Happy path: proxy returns `[text, text]`, we parse it as two
    /// `ContentBlock::Text` and return them in order.
    #[tokio::test]
    async fn drain_notifications_parses_text_blocks_in_order() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notify"))
            .and(header("Mcp-Session-Id", ""))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"type": "text", "text": "first"},
                {"type": "text", "text": "second"},
            ])))
            .mount(&server)
            .await;

        let conn = Connection::new_for_test("t".into(), server.uri());
        let blocks = conn.drain_notifications().await.expect("drain ok");
        assert_eq!(blocks.len(), 2);
        match &blocks[0] {
            ContentBlock::Text(TextContent { text, .. }) => assert_eq!(text, "first"),
            other => panic!("expected text, got {other:?}"),
        }
        match &blocks[1] {
            ContentBlock::Text(TextContent { text, .. }) => assert_eq!(text, "second"),
            other => panic!("expected text, got {other:?}"),
        }
    }

    /// 404 (proxy lost the session, e.g. after a restart) → empty vec
    /// rather than an error. The next upstream call will surface the
    /// session-lost condition through its own error path; init-time
    /// drain shouldn't be the one to abort the request.
    #[tokio::test]
    async fn drain_notifications_404_returns_empty() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notify"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let conn = Connection::new_for_test("t".into(), server.uri());
        let blocks = conn.drain_notifications().await.expect("404 → ok(empty)");
        assert!(blocks.is_empty(), "expected empty vec, got {blocks:?}");
    }

    /// Empty queue → empty array → empty vec. The most common case.
    #[tokio::test]
    async fn drain_notifications_empty_queue_returns_empty() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let conn = Connection::new_for_test("t".into(), server.uri());
        let blocks = conn.drain_notifications().await.expect("drain ok");
        assert!(blocks.is_empty(), "expected empty vec, got {blocks:?}");
    }

    /// Non-success / non-404 status propagates as `BadStatus`.
    #[tokio::test]
    async fn drain_notifications_5xx_returns_bad_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notify"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let conn = Connection::new_for_test("t".into(), server.uri());
        let err = conn
            .drain_notifications()
            .await
            .expect_err("5xx → err");
        match err {
            super::super::Error::BadStatus { code, body, .. } => {
                assert_eq!(code.as_u16(), 500);
                assert_eq!(body, "boom");
            }
            other => panic!("expected BadStatus, got {other:?}"),
        }
    }

    /// Mock connections never hit the network and always return empty.
    #[tokio::test]
    async fn drain_notifications_mock_returns_empty() {
        let conn = Connection::new_mock("http://does-not-matter".into());
        let blocks = conn.drain_notifications().await.expect("mock ok");
        assert!(blocks.is_empty());
    }
}
