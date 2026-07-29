//! MCP connection for communicating with an MCP server.
//!
//! [`Connection`] is a cheaply-clonable handle around an internal
//! [`ConnectionInner`]. The last drop of the inner `Arc` drops
//! [`ConnectionInner`]'s `_listener_cancel_guard` field (a
//! [`tokio_util::sync::DropGuard`]), which cancels the listener task's
//! [`tokio_util::sync::CancellationToken`] — the SSE listener exits the
//! instant any in-flight reconnect, sleep, or read is cancelled, with no
//! zombie 401 retries against a now-dead proxy session.

use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock as StdRwLock, Weak};
use std::time::Duration;

use indexmap::IndexMap;
use tokio::sync::RwLock;
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
/// inner `Arc` ref count hits zero, the `_listener_cancel_guard` field is
/// dropped, and the SSE listener task is
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
///
/// `E` is the command-execution extension's fulfiller: when the server
/// exposed the objectiveai capability, `cli_request` frames arriving on
/// the SSE stream run through it and their results are POSTed back —
/// see [`super::McpClientCommandExecutor`]. The default
/// [`super::NotSupportedMcpClientCommandExecutor`] answers every
/// request with a "not supported" error.
pub struct Connection<
    E: super::McpClientCommandExecutor =
        super::NotSupportedMcpClientCommandExecutor,
> {
    inner: Arc<ConnectionInner<E>>,
}

/// Manual `Debug` (not derived) so `E` need not be `Debug` — holders
/// (e.g. the proxy's `Upstream` enum) derive `Debug` themselves.
impl<E: super::McpClientCommandExecutor> std::fmt::Debug for Connection<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").field("inner", &self.inner).finish()
    }
}

impl<E: super::McpClientCommandExecutor> Clone for Connection<E> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// No `Drop` for `Connection`: cancellation happens deterministically
// when the last `Arc<ConnectionInner>` clone is dropped, which drops
// the `_listener_cancel_guard` field and cancels the listener token.

impl<E: super::McpClientCommandExecutor> Deref for Connection<E> {
    type Target = ConnectionInner<E>;
    fn deref(&self) -> &ConnectionInner<E> {
        &self.inner
    }
}

impl<E: super::McpClientCommandExecutor> Connection<E> {
    /// Tear this connection down explicitly.
    ///
    /// 1. Cancels the long-lived list-changed listener task immediately
    ///    (drops the [`DropGuard`] in
    ///    [`ConnectionInner::_listener_cancel_guard`]), so by the time
    ///    the HTTP DELETE goes out the listener isn't still holding an
    ///    SSE read open against the upstream we're about to close.
    /// 2. Issues `DELETE /` to the upstream with this connection's
    ///    `Mcp-Session-Id` and the same merged header set every other
    ///    RPC stamps. Reuses [`ConnectionInner::call_timeout`].
    /// 3. Treats `404 / 401 / 403` as success — the upstream is
    ///    unreachable from these credentials anyway, which is the
    ///    desired terminal state. Other non-2xx surfaces as
    ///    [`super::Error::BadStatus`].
    ///
    /// Takes `&self`: the listener cancel is in-place, and dropping
    /// the surrounding `Arc<ConnectionInner>` (which closes the rest
    /// of the connection's owned state) is the caller's responsibility
    /// — usually by dropping the `Arc<Session>` holding it. Stateless
    /// callers that don't hold a `Connection` should use
    /// [`Client::delete`](super::Client::delete) instead.
    ///
    /// **In-flight RPC ordering.** This method does not block on
    /// in-flight `call_tool` / `read_resource` / `list_tools` /
    /// `list_resources` calls on the same connection. If one is
    /// outstanding when `delete` lands, the upstream may see DELETE
    /// before the RPC's reply makes it back; the in-flight call then
    /// surfaces as a closed-connection error to whoever started it.
    /// That's the spec-correct order (client said terminate) — drain
    /// on the caller side first if you need different semantics.
    pub async fn delete(&self) -> Result<(), super::Error> {
        // 1. Drop the listener-cancel guard. Releasing the `DropGuard`
        //    cancels the sibling `CancellationToken` the listener task
        //    holds; the listener `tokio::select!`s against it on every
        //    blocking await and exits inside one scheduler tick.
        if let Ok(mut guard) = self.inner._listener_cancel_guard.lock() {
            let _ = guard.take();
        }

        // 2. Build + send HTTP DELETE. Mirrors `Client::connect_once`'s
        //    request-stamp shape: header loop first, explicit
        //    `Mcp-Session-Id` always wins.
        let request = super::apply_timeout(
            self.inner.http_client.delete(&self.inner.url),
            self.inner.call_timeout,
        )
        .headers(self.inner.build_request_headers(None, None).await);
        let response = request.send().await.map_err(|source| {
            super::Error::Request {
                url: self.inner.url.clone(),
                source,
            }
        })?;

        // 3. 404 / 401 / 403 → success; other non-2xx → real error.
        let status = response.status();
        if matches!(
            status,
            reqwest::StatusCode::NOT_FOUND
                | reqwest::StatusCode::UNAUTHORIZED
                | reqwest::StatusCode::FORBIDDEN
        ) {
            return Ok(());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(super::Error::BadStatus {
                url: self.inner.url.clone(),
                code: status,
                body: body.chars().take(800).collect(),
            });
        }
        Ok(())
    }

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
        call_timeout: Option<Duration>,
        initialize_result: super::initialize_result::InitializeResult,
        initial_sse_lines: Option<super::LinesStream>,
        executor: E,
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
            executor,
        )
        .await;
        Self { inner }
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
    ) -> Result<crate::agent::completions::message::ToolMessage, super::Error>
    {
        self.inner.call_tool_as_message(params, tool_call_id).await
    }

    /// Returns all resources from the upstream server.
    pub async fn list_resources(
        &self,
    ) -> Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>> {
        self.inner.list_resources().await
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

    /// Atomically replace the connection's [`ConnectionInner::extra_headers`]
    /// bag. Every subsequent outbound HTTP request from this connection
    /// stamps the new map AFTER `headers`, with `HeaderMap::insert`
    /// REPLACE semantics — keys in `extras` override the same key in
    /// the per-URL `headers` bag set at `Client::connect`. Caller
    /// supplies the FULL replacement map; missing keys are dropped
    /// (no merge).
    ///
    /// Used by the proxy to inject session-global headers
    /// (`X-OBJECTIVEAI-RESPONSE-ID`, `X-OBJECTIVEAI-RESPONSE-IDS`)
    /// that re-set on every inbound `initialize`, without re-dialing
    /// the upstream.
    pub async fn set_extra_headers(
        &self,
        extras: IndexMap<String, String>,
    ) {
        *self.inner.extra_headers.write().await = extras;
    }
}

/// Test constructors live on the DEFAULT-executor `Connection` — the
/// capability-gate tests never execute commands.
#[cfg(test)]
impl Connection {
    pub(crate) fn new_for_test(name: String, url: String) -> Self {
        Self {
            inner: ConnectionInner::new_for_test(name, url),
        }
    }

    pub(crate) fn new_for_test_with_caps(
        name: String,
        url: String,
        capabilities: super::initialize_result::ServerCapabilities,
    ) -> Self {
        Self {
            inner: ConnectionInner::new_for_test_with_caps(
                name,
                url,
                capabilities,
            ),
        }
    }
}

/// The actual connection state. Behind an `Arc` inside [`Connection`].
///
/// Fields are public for read-only access (callers reach them via
/// `Connection`'s `Deref`), but every method on this type is private —
/// the public surface lives on [`Connection`] and delegates through.
pub struct ConnectionInner<
    E: super::McpClientCommandExecutor =
        super::NotSupportedMcpClientCommandExecutor,
> {
    pub http_client: reqwest::Client,
    pub url: String,
    pub session_id: String,
    /// All HTTP headers stamped on every POST / GET this connection
    /// makes — the same merged map (defaults + caller overrides) the
    /// `Client` built once during connect. `Mcp-Session-Id`,
    /// `Content-Type`, and `Accept` are still set by the request
    /// builders and override anything in `headers`.
    pub headers: IndexMap<String, String>,
    /// Mutable per-request override layer stamped AFTER `headers` on
    /// every outbound HTTP request. The request-builder uses
    /// `reqwest::header::HeaderMap::insert` semantics so any key
    /// present in `extra_headers` REPLACES the same key in `headers`.
    /// Used by the proxy to inject session-global headers
    /// (`X-OBJECTIVEAI-RESPONSE-ID` etc.) that override per-URL
    /// values without re-dialing. Empty by default; set via
    /// [`Connection::set_extra_headers`].
    pub extra_headers: RwLock<IndexMap<String, String>>,

    pub backoff_current_interval: Duration,
    pub backoff_initial_interval: Duration,
    pub backoff_randomization_factor: f64,
    pub backoff_multiplier: f64,
    pub backoff_max_interval: Duration,
    pub backoff_max_elapsed_time: Duration,
    /// Per-RPC timeout; `None` = no timeout (wait forever).
    pub call_timeout: Option<Duration>,

    /// The server's capabilities and info from the initialize response.
    pub initialize_result: super::initialize_result::InitializeResult,

    /// Auto-incrementing request ID (starts at 2; 1 was used for initialize).
    next_id: AtomicU64,

    /// All tools from the server, populated by background pagination.
    ///
    /// `None` = cache cleared — either pre-populate (between
    /// [`Self::new`] and the first `refresh_tools_signaling`) or
    /// post-drop (the listener empties this the moment its SSE
    /// stream ends so `list_tools` will re-paginate against the
    /// upstream rather than return stale state). `Some(_)` = last
    /// known result, `Ok` or `Err`.
    tools:
        RwLock<Option<Result<Arc<Vec<super::tool::Tool>>, Arc<super::Error>>>>,
    /// All resources from the server, populated by background pagination.
    /// Same `None`/`Some` semantics as [`Self::tools`].
    resources: RwLock<
        Option<Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>>>,
    >,

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
    ///
    /// Wrapped in `Mutex<Option<_>>` so explicit teardown paths
    /// ([`Connection::delete`]) can drop the guard in place — firing
    /// the cancel token *before* the surrounding `Arc<ConnectionInner>`
    /// goes away. Regular drop still works: `Mutex<Option<DropGuard>>`
    /// drops its inner `DropGuard` automatically when the mutex itself
    /// drops, so the listener is still cancelled on the last `Arc` drop.
    _listener_cancel_guard: std::sync::Mutex<Option<DropGuard>>,

    /// Optional callback fired *after* the listener has refreshed the
    /// tool cache in response to an upstream `notifications/tools/list_changed`.
    /// Set via [`Connection::set_on_tools_list_changed`].
    on_tools_list_changed: CallbackSlot,

    /// Optional callback fired *after* the listener has refreshed the
    /// resource cache in response to an upstream
    /// `notifications/resources/list_changed`.
    /// Set via [`Connection::set_on_resources_list_changed`].
    on_resources_list_changed: CallbackSlot,

    /// Fulfiller for the command-execution extension. When the server
    /// exposed the objectiveai capability, the SSE listener runs each
    /// inbound `cli_request` through this and POSTs the results back —
    /// see [`Self::fulfill_cli_request`].
    executor: E,
}

/// Manual `Debug` (not derived) so `E` need not be `Debug`.
impl<E: super::McpClientCommandExecutor> std::fmt::Debug
    for ConnectionInner<E>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionInner")
            .field("url", &self.url)
            .field("session_id", &self.session_id)
            .field("initialize_result", &self.initialize_result)
            .finish_non_exhaustive()
    }
}

/// Test constructors on the DEFAULT-executor inner — mirrors the
/// `#[cfg(test)] impl Connection` block above.
#[cfg(test)]
impl ConnectionInner {
    /// Creates a minimal connection for unit testing. Declares both
    /// `tools` and `resources` capabilities with `list_changed:
    /// Some(true)` so callers exercise the present-cap +
    /// list_changed-enabled paths in `list_*`, `refresh_*`, and
    /// `subscribe_*`. For other capability shapes use
    /// `new_for_test_with_caps`.
    fn new_for_test(name: String, url: String) -> Arc<Self> {
        Self::new_for_test_with_caps(
            name,
            url,
            super::initialize_result::ServerCapabilities {
                experimental: None,
                logging: None,
                completions: None,
                prompts: None,
                resources: Some(
                    super::initialize_result::ResourcesCapability {
                        subscribe: None,
                        list_changed: Some(true),
                    },
                ),
                tools: Some(super::initialize_result::ToolsCapability {
                    list_changed: Some(true),
                }),
                tasks: None,
            },
        )
    }

    /// Creates a minimal connection for unit testing with an explicit
    /// `ServerCapabilities`. Used by the capability-gating tests to
    /// drive each gate's absent-cap branch.
    fn new_for_test_with_caps(
        name: String,
        url: String,
        capabilities: super::initialize_result::ServerCapabilities,
    ) -> Arc<Self> {
        Arc::new(Self {
            http_client: reqwest::Client::new(),
            url,
            session_id: String::new(),
            headers: IndexMap::new(),
            extra_headers: RwLock::new(IndexMap::new()),
            backoff_current_interval: Duration::from_millis(500),
            backoff_initial_interval: Duration::from_millis(500),
            backoff_randomization_factor: 0.5,
            backoff_multiplier: 1.5,
            backoff_max_interval: Duration::from_secs(60),
            backoff_max_elapsed_time: Duration::from_secs(900),
            call_timeout: Some(Duration::from_secs(30)),
            initialize_result: super::initialize_result::InitializeResult {
                protocol_version: "2025-03-26".into(),
                capabilities,
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
            next_id: AtomicU64::new(2),
            // Test connection has no listener and never refreshes; seed
            // with an empty Ok so `list_tools` doesn't try to paginate.
            tools: RwLock::new(Some(Ok(Arc::new(Vec::new())))),
            resources: RwLock::new(Some(Ok(Arc::new(Vec::new())))),
            _listener_cancel_guard: std::sync::Mutex::new(None),
            on_tools_list_changed: CallbackSlot::new(),
            on_resources_list_changed: CallbackSlot::new(),
            executor: super::NotSupportedMcpClientCommandExecutor,
        })
    }
}

impl<E: super::McpClientCommandExecutor> ConnectionInner<E> {
    /// Creates a new connection and spawns background tasks to paginate
    /// all tools and resources. Called internally by
    /// [`Client::connect`](super::Client::connect) (via [`Connection::new`]).
    ///
    /// `initial_sse_lines`, if `Some`, is a pre-opened SSE line reader
    /// that the list-changed listener will read from immediately on its
    /// first iteration, instead of opening its own GET `/`. The caller
    /// is responsible for arranging for one of these to exist whenever
    /// the upstream advertises `tools.list_changed` or
    /// `resources.list_changed`, or the objectiveai command-execution
    /// capability — see [`Client::connect`](super::Client::connect).
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
        call_timeout: Option<Duration>,
        initialize_result: super::initialize_result::InitializeResult,
        initial_sse_lines: Option<super::LinesStream>,
        executor: E,
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
            extra_headers: RwLock::new(IndexMap::new()),
            backoff_current_interval,
            backoff_initial_interval,
            backoff_randomization_factor,
            backoff_multiplier,
            backoff_max_interval,
            backoff_max_elapsed_time,
            call_timeout,
            initialize_result,
            next_id: AtomicU64::new(2),
            // Start empty; `refresh_tools_signaling` below installs
            // `Some(_)` before `new` returns (the lock-handoff oneshot
            // gates the return on the writer holding the lock).
            tools: RwLock::new(None),
            resources: RwLock::new(None),
            _listener_cancel_guard: std::sync::Mutex::new(Some(
                listener_cancel.drop_guard(),
            )),
            on_tools_list_changed: CallbackSlot::new(),
            on_resources_list_changed: CallbackSlot::new(),
            executor,
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

    /// Server declared a `tools` capability in its `InitializeResult`.
    /// Gates `list_tools`, `call_tool`, `refresh_tools`. When `false` the
    /// upstream cannot service `tools/*` RPCs at all and any attempt would
    /// either hang in idempotent backoff (`tools/list`) or fail with
    /// `SessionExpired` (`tools/call`).
    fn has_tools_cap(&self) -> bool {
        self.initialize_result.capabilities.tools.is_some()
    }

    /// Server declared a `resources` capability in its `InitializeResult`.
    /// Gates `list_resources`, `read_resource`, `refresh_resources`, and
    /// the post-`call_tool` ResourceLink resolution. Same hang-or-fail
    /// shape as `has_tools_cap` when absent.
    fn has_resources_cap(&self) -> bool {
        self.initialize_result.capabilities.resources.is_some()
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

    /// Builds a POST request with all required headers and the call
    /// timeout (when one is configured).
    async fn post(&self) -> reqwest::RequestBuilder {
        super::apply_timeout(self.http_client.post(&self.url), self.call_timeout)
            .headers(
                self.build_request_headers(
                    Some("application/json"),
                    Some("application/json, text/event-stream"),
                )
                .await,
            )
    }

    /// Build the `HeaderMap` stamped on every outbound request. Order
    /// of insertion drives override semantics — `HeaderMap::insert`
    /// REPLACES existing values for the same key:
    ///
    /// 1. Content-Type / Accept (when supplied by the caller).
    /// 2. `self.headers` (the per-URL bag set at `Client::connect`).
    /// 3. `self.extra_headers` (the mutable, session-global overrides
    ///    — proxies use this for `X-OBJECTIVEAI-RESPONSE-ID` etc).
    /// 4. `Mcp-Session-Id` (the connection's own session id, always
    ///    last so it can never be shadowed).
    async fn build_request_headers(
        &self,
        content_type: Option<&str>,
        accept: Option<&str>,
    ) -> reqwest::header::HeaderMap {
        use reqwest::header::{
            ACCEPT, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue,
        };
        let mut hmap = HeaderMap::new();
        if let Some(ct) = content_type {
            if let Ok(hv) = HeaderValue::from_str(ct) {
                hmap.insert(CONTENT_TYPE, hv);
            }
        }
        if let Some(a) = accept {
            if let Ok(hv) = HeaderValue::from_str(a) {
                hmap.insert(ACCEPT, hv);
            }
        }
        for (k, v) in &self.headers {
            if let (Ok(hn), Ok(hv)) = (
                HeaderName::try_from(k.as_str()),
                HeaderValue::from_str(v),
            ) {
                hmap.insert(hn, hv);
            }
        }
        let extras = self.extra_headers.read().await;
        for (k, v) in extras.iter() {
            if let (Ok(hn), Ok(hv)) = (
                HeaderName::try_from(k.as_str()),
                HeaderValue::from_str(v),
            ) {
                hmap.insert(hn, hv);
            }
        }
        drop(extras);
        if let Ok(hv) = HeaderValue::from_str(&self.session_id) {
            hmap.insert(HeaderName::from_static("mcp-session-id"), hv);
        }
        hmap
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
    /// Mint the next request id from the connection's counter.
    fn next_request_id(&self) -> super::RequestId {
        super::RequestId::Number(
            self.next_id.fetch_add(1, Ordering::Relaxed).into(),
        )
    }

    async fn rpc<R: serde::de::DeserializeOwned>(
        &self,
        body: super::JsonRpcRequest,
        idempotent: bool,
    ) -> Result<R, super::Error> {

        let attempt_one = || async {
            let url = self.url.clone();
            let response =
                self.post().await.json(&body).send().await.map_err(|source| {
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
                    super::Error::BadStatus {
                        url: url.clone(),
                        code,
                        body,
                    },
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
                backoff::Error::Permanent(err)
                | backoff::Error::Transient { err, .. } => err,
            })
        }
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
            super::JsonRpcRequest::list_tools(
                self.next_request_id(),
                super::tool::ListToolsRequest {
                    cursor: cursor.map(String::from),
                },
            ),
            true,
        )
        .await
    }

    /// Returns all tools from the server.
    ///
    /// Blocks until background pagination completes, then returns a
    /// cheap `Arc` clone of the result. If the cache is currently
    /// empty (e.g. because the listener detected its SSE stream drop
    /// and cleared it) this paginates inline against the upstream —
    /// the caller gets fresh data on the happy path, or the live
    /// upstream error if the connection is genuinely down, rather
    /// than stale pre-drop tools.
    async fn list_tools(
        &self,
    ) -> Result<Arc<Vec<super::tool::Tool>>, Arc<super::Error>> {
        if !self.has_tools_cap() {
            return Ok(Arc::new(Vec::new()));
        }
        if let Some(cached) = self.tools.read().await.as_ref() {
            return cached.clone();
        }
        // Cache cleared; refresh inline. Concurrent callers may each
        // refresh — wasteful, but the proxy fans out across distinct
        // upstreams so a single Connection rarely sees concurrent
        // `list_tools` calls.
        self.refresh_tools(None).await;
        self.tools
            .read()
            .await
            .as_ref()
            .expect("refresh_tools installs Some")
            .clone()
    }

    /// Calls a tool on the MCP server. The returned
    /// `CallToolResult.content` is **fully resolved**: every
    /// `ContentBlock::ResourceLink { uri }` whose URI appears in
    /// `list_resources` has already been replaced by one or more
    /// `ContentBlock::EmbeddedResource` blocks carrying the fetched
    /// contents (via `read_resource`). Unknown-URI links pass through
    /// untouched — the upstream server may have its own out-of-band
    /// resolution path the caller should preserve.
    ///
    /// Resolving inside `call_tool` (rather than downstream in
    /// `call_tool_as_message`) means every consumer of the result
    /// sees `EmbeddedResource` shapes uniformly; the stateless
    /// `From<ContentBlock>` impl is then enough to convert the whole
    /// result to `RichContent` with no further connection work.
    async fn call_tool(
        &self,
        params: &super::tool::CallToolRequestParams,
    ) -> Result<super::tool::CallToolResult, super::Error> {
        if !self.has_tools_cap() {
            return Err(super::Error::UnsupportedCapability {
                capability: "tools",
            });
        }
        let mut result: super::tool::CallToolResult = self
            .rpc(
                super::JsonRpcRequest::call_tool(
                    self.next_request_id(),
                    params.clone(),
                ),
                false,
            )
            .await?;

        // Build the known-resource URI set for ResourceLink
        // resolution. `list_resources` failure → empty set (same
        // safe fallback the resolution path used previously).
        let known_uris: std::collections::HashSet<String> =
            match self.list_resources().await {
                Ok(rs) => rs.iter().map(|r| r.uri.clone()).collect(),
                Err(_) => std::collections::HashSet::new(),
            };

        // Walk the blocks, replacing each resolvable ResourceLink
        // with one EmbeddedResource per returned ResourceContentsUnion.
        // Everything else (Text, Image, Audio, EmbeddedResource,
        // unknown-URI ResourceLinks) passes through.
        let mut resolved: Vec<super::tool::ContentBlock> =
            Vec::with_capacity(result.content.len());
        for block in std::mem::take(&mut result.content) {
            match block {
                super::tool::ContentBlock::ResourceLink(link)
                    if known_uris.contains(&link.uri) =>
                {
                    let read = self.read_resource(&link.uri).await?;
                    for contents in read.contents {
                        resolved.push(
                            super::tool::ContentBlock::EmbeddedResource(
                                super::tool::EmbeddedResource {
                                    resource: contents,
                                    // ResourceLink's annotations
                                    // don't have a perfect home on
                                    // EmbeddedResource — both fields
                                    // exist but they describe different
                                    // shapes (the link vs the inlined
                                    // contents). Drop them on the way
                                    // in; the EmbeddedResource is now
                                    // a fresh authoritative block.
                                    annotations: None,
                                    _meta: None,
                                },
                            ),
                        );
                    }
                }
                other => resolved.push(other),
            }
        }
        result.content = resolved;
        Ok(result)
    }

    /// Calls a tool and converts the (already-resolved) result into a
    /// [`ToolMessage`]. Resource resolution happens inside
    /// [`Self::call_tool`] — by the time we get the blocks here every
    /// resolvable `ResourceLink` has already been replaced with an
    /// `EmbeddedResource`, so the conversion is a pure stateless
    /// element-wise map through [`From<ContentBlock> for
    /// RichContentPart`](crate::agent::completions::message::RichContentPart).
    ///
    /// Content-block mapping (handled by the `From` impl):
    /// - `text` → text part
    /// - `image` → image_url part (data URL)
    /// - `audio` → input_audio part
    /// - `embedded_resource` (text) → text part
    /// - `embedded_resource` (blob, image mime) → image_url part
    /// - `embedded_resource` (blob, audio mime) → input_audio part
    /// - `embedded_resource` (blob, video mime) → input_video part
    /// - `embedded_resource` (blob, other mime) → file part
    /// - `resource_link` (unknown URI) → JSON-text fallback
    ///   (resolvable URIs were already resolved upstream)
    async fn call_tool_as_message(
        &self,
        params: &super::tool::CallToolRequestParams,
        tool_call_id: String,
    ) -> Result<crate::agent::completions::message::ToolMessage, super::Error>
    {
        use crate::agent::completions::message::{
            RichContentPart, ToolMessage, ToolResponseMetadata,
        };

        let result = self.call_tool(params).await?;

        let parts: Vec<RichContentPart> =
            result.content.into_iter().map(Into::into).collect();

        // Lossy-decode the MCP `_meta` extension bag into our typed
        // `ToolResponseMetadata`. Unknown keys (set by non-objectiveai
        // upstreams) are silently dropped. Decoding failure leaves
        // metadata as `None`.
        let metadata = result._meta.as_ref().and_then(|m| {
            serde_json::from_value::<ToolResponseMetadata>(
                serde_json::to_value(m).ok()?,
            )
            .ok()
        });

        Ok(ToolMessage {
            content: parts.into(),
            tool_call_id,
            metadata,
        })
    }

    /// Sends a `resources/list` RPC call for a single page.
    async fn rpc_list_resources(
        &self,
        cursor: Option<&str>,
    ) -> Result<super::resource::ListResourcesResult, super::Error> {
        self.rpc(
            super::JsonRpcRequest::list_resources(
                self.next_request_id(),
                super::resource::ListResourcesRequest {
                    cursor: cursor.map(String::from),
                },
            ),
            true,
        )
        .await
    }

    /// Returns all resources from the server.
    ///
    /// Same cache-or-refresh semantics as [`Self::list_tools`]: a
    /// cleared cache (post-drop or pre-populate) triggers an inline
    /// paginate against the upstream.
    async fn list_resources(
        &self,
    ) -> Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>> {
        if !self.has_resources_cap() {
            return Ok(Arc::new(Vec::new()));
        }
        if let Some(cached) = self.resources.read().await.as_ref() {
            return cached.clone();
        }
        self.refresh_resources(None).await;
        self.resources
            .read()
            .await
            .as_ref()
            .expect("refresh_resources installs Some")
            .clone()
    }

    /// Reads a resource from the MCP server.
    async fn read_resource(
        &self,
        uri: &str,
    ) -> Result<super::resource::ReadResourceResult, super::Error> {
        if !self.has_resources_cap() {
            return Err(super::Error::UnsupportedCapability {
                capability: "resources",
            });
        }
        self.rpc(
            super::JsonRpcRequest::read_resource(
                self.next_request_id(),
                super::resource::ReadResourceRequestParams {
                    uri: uri.to_string(),
                },
            ),
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
        if !self.has_tools_cap() {
            // No tools capability — install an empty Vec so the cache
            // contract holds (`list_tools`'s `.expect("refresh_tools
            // installs Some")` etc.) and return without paginating or
            // signalling. No `notify_waiters` and no `on_change` —
            // nothing real changed.
            let mut guard = self.tools.write().await;
            *guard = Some(Ok(Arc::new(Vec::new())));
            return;
        }
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
        // `on_change` fires under the write guard, *after* `*guard =
        // result`, so anyone the callback wakes queues on the read lock,
        // waits for the guard to drop, and observes the post-swap state.
        let (mut guard, result) =
            tokio::join!(self.tools.write(), self.paginate_tools(),);
        *guard = Some(result);
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
    ///
    /// **Caller invariant:** the spawn site in `ConnectionInner::new`
    /// must gate this call on `capabilities.tools.is_some()`. This
    /// method assumes the tools capability is present and issues
    /// `tools/list` RPCs unconditionally — running it against a
    /// no-tools server triggers the 15-min idempotent-backoff storm.
    async fn refresh_tools_signaling(
        &self,
        lock_held: tokio::sync::oneshot::Sender<()>,
        on_change: Option<ListChangedCallback>,
    ) {
        let mut guard = self.tools.write().await;
        // Signal `lock_held` while we hold the write lock and *before*
        // installing the new list, so the `on_change` callback observes
        // the "list change in flight" edge — its readers queue behind
        // this write guard and always see the post-swap state.
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
        *guard = Some(result);
    }

    /// Re-fetches all resources from the server, replacing the cached list.
    /// See [`ConnectionInner::refresh_tools`] for the callback timing
    /// contract.
    async fn refresh_resources(&self, on_change: Option<ListChangedCallback>) {
        if !self.has_resources_cap() {
            // Symmetric to `refresh_tools` — see that gate.
            let mut guard = self.resources.write().await;
            *guard = Some(Ok(Arc::new(Vec::new())));
            return;
        }
        // Same paginate-while-acquiring-the-write-lock pattern as
        // `refresh_tools` — see that comment for the visibility +
        // performance rationale.
        let (mut guard, result) =
            tokio::join!(self.resources.write(), self.paginate_resources(),);
        *guard = Some(result);
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

    /// Resource counterpart of [`Self::refresh_tools_signaling`]. The
    /// same spawn-site-gate invariant applies: the caller must gate
    /// the spawn on `capabilities.resources.is_some()`.
    async fn refresh_resources_signaling(
        &self,
        lock_held: tokio::sync::oneshot::Sender<()>,
        on_change: Option<ListChangedCallback>,
    ) {
        let mut guard = self.resources.write().await;
        // See `refresh_tools_signaling` — signal `lock_held` under the
        // write lock, before install, so the `on_change` callback's
        // readers see the post-swap state.
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
        *guard = Some(result);
    }

    /// Fulfill one command-execution request from the server: run it
    /// through the connection's executor and POST every resulting
    /// frame to `{url}/objectiveai/command`, in stream order, one POST
    /// per frame, each carrying the request's correlation id.
    ///
    /// Frame discipline (see [`super::CliResponse`]): the exchange
    /// ALWAYS opens with an `Ack` — POSTed before the run starts, so
    /// the server knows a response is coming even when the run is
    /// slow — and ALWAYS ends with a `Done`, even when the run failed
    /// to start or the pump aborted. Stream errors are NON-terminal
    /// (`Error` frames — the stream may keep yielding). A POST
    /// transport failure aborts the pump — dropping the stream cancels
    /// the run — but the final `Done` is still attempted (its own
    /// failure is ignored: there is nobody left to tell, and no
    /// logging surface to tell them on). An undeliverable `Ack` skips
    /// the run entirely: the server is unreachable, so the output
    /// would be undeliverable too.
    ///
    /// Spawned from the SSE listener — requests are fulfilled in
    /// PARALLEL (a long run never delays other notifications). Frame
    /// order is guaranteed per run; ordering ACROSS concurrent runs is
    /// not.
    async fn fulfill_cli_request(&self, params: super::CliRequestParams) {
        use futures_util::StreamExt;
        let endpoint = format!(
            "{}{}",
            self.url.trim_end_matches('/'),
            super::CLI_COMMAND_ENDPOINT_SUFFIX,
        );
        let id = params.id;
        // Opener — before execute(), which may be slow to even start.
        if self
            .post_cli_response(
                &endpoint,
                &super::CliResponse::Ack { id: id.clone() },
            )
            .await
            .is_err()
        {
            let _ = self
                .post_cli_response(
                    &endpoint,
                    &super::CliResponse::Done { id },
                )
                .await;
            return;
        }
        match self.executor.execute(params.request).await {
            Err(e) => {
                let _ = self
                    .post_cli_response(
                        &endpoint,
                        &super::CliResponse::Error {
                            id: id.clone(),
                            error: e.to_string(),
                        },
                    )
                    .await;
            }
            Ok(stream) => {
                let mut stream = std::pin::pin!(stream);
                while let Some(result) = stream.next().await {
                    let frame = match result {
                        // Serialized HERE, where the concrete item type
                        // is still known — the frame carries raw JSON
                        // so the receiver never round-trips it through
                        // the untagged sum (see `CliResponse::Item`).
                        Ok(item) => match serde_json::to_value(&item) {
                            Ok(item) => super::CliResponse::Item {
                                id: id.clone(),
                                item,
                            },
                            Err(e) => super::CliResponse::Error {
                                id: id.clone(),
                                error: e.to_string(),
                            },
                        },
                        Err(e) => super::CliResponse::Error {
                            id: id.clone(),
                            error: e.to_string(),
                        },
                    };
                    if self
                        .post_cli_response(&endpoint, &frame)
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
        let _ = self
            .post_cli_response(&endpoint, &super::CliResponse::Done { id })
            .await;
    }

    /// POST one [`super::CliResponse`] frame to the command-response
    /// endpoint. `Ok(())` on any 2xx; every failure mode (connect,
    /// timeout, non-2xx) collapses to `Err(())` — the pump's only
    /// decision is "keep going or abort".
    async fn post_cli_response(
        &self,
        endpoint: &str,
        frame: &super::CliResponse,
    ) -> Result<(), ()> {
        let request = super::apply_timeout(
            self.http_client.post(endpoint),
            self.call_timeout,
        )
        .headers(
            self.build_request_headers(
                Some("application/json"),
                Some("application/json"),
            )
            .await,
        )
        .json(frame);
        match request.send().await {
            Ok(response) if response.status().is_success() => Ok(()),
            _ => Err(()),
        }
    }

    /// Builds a GET request to the MCP endpoint for receiving server
    /// notifications via SSE.
    async fn get(&self) -> reqwest::RequestBuilder {
        self.http_client
            .get(&self.url)
            .headers(
                self.build_request_headers(None, Some("text/event-stream"))
                    .await,
            )
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

        // The stream's SSE last-event-id, updated from every `id:`
        // line. Reconnects send it as the `Last-Event-ID` header so a
        // spec-conformant server (rmcp caches server→client frames in
        // a ring buffer) resumes PRECISELY after the last event we
        // processed. Without it, rmcp treats a bare GET as
        // resume-from-0 and replays its whole retained cache — for
        // list_changed that's harmless, but a replayed `cli_request`
        // would re-execute a command we already ran.
        let mut last_event_id: Option<String> = None;

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
                        out = async {
                            let mut request = this.get().await;
                            if let Some(id) = last_event_id.as_deref() {
                                request =
                                    request.header("Last-Event-ID", id);
                            }
                            request.send().await
                        } => out,
                        _ = cancel.cancelled() => {
                            drop(this);
                            return;
                        }
                    };
                    let response = match send_outcome {
                        Ok(r) if r.status().is_success() => r,
                        outcome => {
                            // A definite HTTP rejection may mean the
                            // server no longer honors our
                            // Last-Event-ID (index evicted from its
                            // cache, restarted session) — drop it so
                            // the next attempt reconnects plain
                            // instead of retrying a permanently
                            // rejected resume forever. Transport
                            // errors keep it: the id may still be
                            // good once the network recovers.
                            if outcome.is_ok() {
                                last_event_id = None;
                            }
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
                // Spawned like every other handler — the read loop
                // starts immediately (we're already resubscribed, so
                // nothing is lost: a notification landing during the
                // catch-up just spawns its own refresh). tools and
                // resources are independent locks; the two catch-ups
                // run concurrently inside the task.
                let conn = Arc::clone(&this);
                let cancel = cancel.clone();
                tokio::spawn(async move {
                    tokio::select! {
                        _ = async {
                            let tools_cb = conn.on_tools_list_changed.get();
                            let resources_cb =
                                conn.on_resources_list_changed.get();
                            let _ = tokio::join!(
                                conn.refresh_tools(tools_cb),
                                conn.refresh_resources(resources_cb),
                            );
                        } => {}
                        _ = cancel.cancelled() => {}
                    }
                });
            }
            is_reconnect = true;

            'inner: loop {
                tokio::select! {
                    line_result = lines.next_line() => {
                        match line_result {
                            Ok(Some(line)) => {
                                // SSE `id:` lines set the stream's
                                // last-event-id — remember it (empty
                                // value ignored) for precise resume
                                // on reconnect.
                                if let Some(id) = line.strip_prefix("id:") {
                                    let id = id.trim();
                                    if !id.is_empty() {
                                        last_event_id =
                                            Some(id.to_string());
                                    }
                                    continue 'inner;
                                }
                                // SSE data lines start with "data: ".
                                let Some(data) = line.strip_prefix("data: ") else {
                                    continue 'inner;
                                };
                                let notification = match serde_json::from_str::<super::JsonRpcServerNotification>(data) {
                                    Ok(n) => n,
                                    Err(_) => continue 'inner,
                                };
                                // EVERY handler is SPAWNED — the
                                // listener never blocks on one, so
                                // notifications are handled in
                                // parallel and a long-running command
                                // run can't delay a list_changed
                                // refresh (or another command).
                                //
                                // Spawned tasks hold a strong
                                // `Arc<ConnectionInner>` for their
                                // duration, so a plainly-dropped
                                // connection stays alive until its
                                // in-flight handlers finish; explicit
                                // teardown (`Connection::delete`)
                                // fires the cancel token, which every
                                // task races against and aborts on.
                                match notification {
                                    super::JsonRpcServerNotification::ToolsListChanged { .. } => {
                                        // refresh_tools fires the
                                        // callback after the cache is
                                        // installed, so the proxy's
                                        // downstream
                                        // notifications/tools/list_changed
                                        // emission lines up with the
                                        // staleness window opening.
                                        let conn = Arc::clone(&this);
                                        let cancel = cancel.clone();
                                        tokio::spawn(async move {
                                            tokio::select! {
                                                _ = async {
                                                    let cb = conn.on_tools_list_changed.get();
                                                    conn.refresh_tools(cb).await;
                                                } => {}
                                                _ = cancel.cancelled() => {}
                                            }
                                        });
                                    }
                                    super::JsonRpcServerNotification::ResourcesListChanged { .. } => {
                                        let conn = Arc::clone(&this);
                                        let cancel = cancel.clone();
                                        tokio::spawn(async move {
                                            tokio::select! {
                                                _ = async {
                                                    let cb = conn.on_resources_list_changed.get();
                                                    conn.refresh_resources(cb).await;
                                                } => {}
                                                _ = cancel.cancelled() => {}
                                            }
                                        });
                                    }
                                    // Command-execution extension. The
                                    // executor decides support (the
                                    // default answers with a
                                    // "not supported" Error frame).
                                    super::JsonRpcServerNotification::CliRequest { params, .. } => {
                                        let conn = Arc::clone(&this);
                                        let cancel = cancel.clone();
                                        tokio::spawn(async move {
                                            tokio::select! {
                                                _ = conn.fulfill_cli_request(params) => {}
                                                _ = cancel.cancelled() => {}
                                            }
                                        });
                                    }
                                    super::JsonRpcServerNotification::Fallback { .. } => {}
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

            // Stream dropped — empty the per-Connection caches so the
            // next `list_tools` / `list_resources` paginates inline
            // against the (possibly still-dead) upstream rather than
            // returning whatever was cached before the drop. The
            // `is_reconnect` catch-up at the top of the next outer
            // iteration will repopulate `Some(_)` if the reconnect
            // succeeds; if the reconnect keeps failing, the cache
            // stays `None` and `list_*` callers paginate themselves.
            *this.tools.write().await = None;
            *this.resources.write().await = None;

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
mod capability_gate_tests {
    use super::*;
    use crate::mcp::initialize_result::{
        ResourcesCapability, ServerCapabilities, ToolsCapability,
    };
    use crate::mcp::tool::{
        CallToolRequestParams, Tool, ToolSchemaObject, ToolSchemaType,
    };

    /// Builds a `ServerCapabilities` with the given `tools` / `resources`
    /// shapes and every other capability set to `None`. Each gate test
    /// passes its own combination to exercise a specific cap-absent
    /// branch.
    fn caps(
        tools: Option<ToolsCapability>,
        resources: Option<ResourcesCapability>,
    ) -> ServerCapabilities {
        ServerCapabilities {
            experimental: None,
            logging: None,
            completions: None,
            prompts: None,
            resources,
            tools,
            tasks: None,
        }
    }

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

    /// The objectiveai command-execution capability is the presence of
    /// the `"objectiveai"` key in `experimental` — other keys don't
    /// count, absence doesn't count.
    #[test]
    fn objectiveai_capability_is_the_experimental_key() {
        let mut capabilities = caps(None, None);
        assert!(!capabilities.has_objectiveai());

        let mut experimental = IndexMap::new();
        experimental
            .insert("objectiveai".to_string(), serde_json::json!({}));
        capabilities.experimental = Some(experimental);
        assert!(capabilities.has_objectiveai());

        let mut experimental = IndexMap::new();
        experimental.insert("other".to_string(), serde_json::json!({}));
        capabilities.experimental = Some(experimental);
        assert!(!capabilities.has_objectiveai());
    }

    /// 3.1 — `list_tools` short-circuits to `Ok(empty)` when the server
    /// declared no `tools` capability, *before* the cache is consulted.
    /// We poison the cache with `Err` to prove the gate fires first.
    #[tokio::test]
    async fn list_tools_returns_empty_when_tools_cap_absent() {
        let conn = Connection::new_for_test_with_caps(
            "t".into(),
            "http://x".into(),
            caps(None, None),
        );
        let err = super::super::Error::NoSessionId {
            url: "http://x".into(),
            body: String::new(),
        };
        *conn.inner.tools.write().await = Some(Err(Arc::new(err)));

        let got = conn.list_tools().await.unwrap();
        assert!(got.is_empty());
    }

    /// 3.2 — symmetric to 3.1 for `list_resources`.
    #[tokio::test]
    async fn list_resources_returns_empty_when_resources_cap_absent() {
        let conn = Connection::new_for_test_with_caps(
            "t".into(),
            "http://x".into(),
            caps(None, None),
        );
        let err = super::super::Error::NoSessionId {
            url: "http://x".into(),
            body: String::new(),
        };
        *conn.inner.resources.write().await = Some(Err(Arc::new(err)));

        let got = conn.list_resources().await.unwrap();
        assert!(got.is_empty());
    }

    /// 3.3 — `read_resource` errors with `UnsupportedCapability` when
    /// the server declared no `resources` capability, without hitting
    /// the network.
    #[tokio::test]
    async fn read_resource_errors_when_resources_cap_absent() {
        let conn = Connection::new_for_test_with_caps(
            "t".into(),
            "http://x".into(),
            caps(None, None),
        );
        let got = conn.read_resource("file://nope").await;
        assert!(matches!(
            got,
            Err(super::super::Error::UnsupportedCapability {
                capability: "resources"
            })
        ));
    }

    /// 3.4 — `call_tool` errors with `UnsupportedCapability` when the
    /// server declared no `tools` capability.
    #[tokio::test]
    async fn call_tool_errors_when_tools_cap_absent() {
        let conn = Connection::new_for_test_with_caps(
            "t".into(),
            "http://x".into(),
            caps(None, None),
        );
        let params = CallToolRequestParams {
            name: "any".into(),
            arguments: None,
            _meta: None,
            task: None,
        };
        let got = conn.call_tool(&params).await;
        assert!(matches!(
            got,
            Err(super::super::Error::UnsupportedCapability {
                capability: "tools"
            })
        ));
    }

    /// 3.5 — `refresh_tools` installs `Some(Ok(empty))` and returns
    /// without paginating when the server declared no `tools`
    /// capability. Clearing the cache first proves the install ran.
    #[tokio::test]
    async fn refresh_tools_installs_empty_when_tools_cap_absent() {
        let conn = Connection::new_for_test_with_caps(
            "t".into(),
            "http://x".into(),
            caps(None, None),
        );
        *conn.inner.tools.write().await = None;

        conn.inner.refresh_tools(None).await;

        let guard = conn.inner.tools.read().await;
        let v = guard
            .as_ref()
            .expect("refresh installed Some")
            .as_ref()
            .expect("refresh installed Ok");
        assert!(v.is_empty());
    }

    /// 3.6 — symmetric to 3.5 for `refresh_resources`.
    #[tokio::test]
    async fn refresh_resources_installs_empty_when_resources_cap_absent() {
        let conn = Connection::new_for_test_with_caps(
            "t".into(),
            "http://x".into(),
            caps(None, None),
        );
        *conn.inner.resources.write().await = None;

        conn.inner.refresh_resources(None).await;

        let guard = conn.inner.resources.read().await;
        let v = guard
            .as_ref()
            .expect("refresh installed Some")
            .as_ref()
            .expect("refresh installed Ok");
        assert!(v.is_empty());
    }

}
