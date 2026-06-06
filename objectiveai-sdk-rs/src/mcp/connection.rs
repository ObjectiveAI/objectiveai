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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock as StdRwLock, Weak};
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
        Self {
            inner: Arc::clone(&self.inner),
        }
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
        // 1. Mark the connection as "used" so the drop-time
        //    orphan-DELETE check (see `ConnectionInner::Drop`) skips
        //    its own fan-out. Without this, a fresh-mint connection
        //    that's explicitly torn down via `delete()` would race
        //    its own drop-time orphan into a duplicate upstream
        //    DELETE.
        self.inner.any_calls.store(true, Ordering::Relaxed);

        // 2. Drop the listener-cancel guard. Releasing the `DropGuard`
        //    cancels the sibling `CancellationToken` the listener task
        //    holds; the listener `tokio::select!`s against it on every
        //    blocking await and exits inside one scheduler tick.
        if let Ok(mut guard) = self.inner._listener_cancel_guard.lock() {
            let _ = guard.take();
        }

        // 3. Build + send HTTP DELETE. Mirrors `Client::connect_once`'s
        //    request-stamp shape: header loop first, explicit
        //    `Mcp-Session-Id` always wins.
        let request = self
            .inner
            .http_client
            .delete(&self.inner.url)
            .timeout(self.inner.call_timeout)
            .headers(self.inner.build_request_headers(None, None).await);
        let response = request.send().await.map_err(|source| {
            super::Error::Request {
                url: self.inner.url.clone(),
                source,
            }
        })?;

        // 4. 404 / 401 / 403 → success; other non-2xx → real error.
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
        call_timeout: Duration,
        initialize_result: super::initialize_result::InitializeResult,
        initial_sse_lines: Option<super::LinesStream>,
        is_reconnect: bool,
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
            is_reconnect,
        )
        .await;
        Self { inner }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(name: String, url: String) -> Self {
        Self {
            inner: ConnectionInner::new_for_test(name, url),
        }
    }

    #[cfg(test)]
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

    /// Non-draining peek at the proxy's `pending_notifications` queue
    /// via `GET /notify/queued`. Returns `true` iff the queue holds at
    /// least one block. Companion to [`Connection::drain_notifications`]
    /// for callers that want to know whether queued blocks exist
    /// without consuming them.
    ///
    /// A 404 from the proxy (session unknown — possible after a proxy
    /// restart) is mapped to `Ok(false)` for the same reason as the
    /// drain path: callers do not need to distinguish "no
    /// notifications" from "lost session" at the use site.
    pub async fn has_pending_notifications(
        &self,
    ) -> Result<bool, super::Error> {
        self.inner.has_pending_notifications().await
    }

    /// `POST <self.url>/notify` against the ObjectiveAI MCP proxy.
    /// Appends `blocks` to the proxy's pending-notifications queue for
    /// this session; they surface as a user message on the next
    /// `tools/call` response (wrapped in a `<system-reminder>` block)
    /// or as the head of the next agent turn when drained between turns.
    ///
    /// Mirror of [`Connection::drain_notifications`] / [`Connection::has_pending_notifications`]
    /// for the inbound side. A 404 from the proxy means the session is
    /// gone — surfaced as `SessionExpired` so callers can distinguish
    /// "session lost" from "delivery failed" at the use site.
    pub async fn enqueue_notifications(
        &self,
        blocks: &[super::tool::ContentBlock],
    ) -> Result<(), super::Error> {
        self.inner.enqueue_notifications(blocks).await
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
    pub call_timeout: Duration,

    /// The server's capabilities and info from the initialize response.
    pub initialize_result: super::initialize_result::InitializeResult,

    /// `true` iff this connection was opened by resuming an existing
    /// upstream session — i.e. [`Client::connect`](super::Client::connect)
    /// was called with `session_id: Some(...)`. Set once at
    /// construction; never mutated.
    ///
    /// Used by the drop-time orphan-DELETE gate in
    /// [`ConnectionInner::Drop`]: reconnects are **excluded** from
    /// orphan DELETE because their `any_calls == false` only means
    /// "this `Connection` instance didn't use it" — an earlier
    /// instance that opened the upstream session may have. Only
    /// freshly-minted connections (`is_reconnect == false`) that no
    /// one used get the drop-time orphan DELETE.
    is_reconnect: bool,

    /// `true` once any [`Self::call_tool`] or [`Self::read_resource`]
    /// has been issued through this connection. Listings
    /// ([`Self::list_tools`], [`Self::list_resources`]) and the
    /// proxy-side notification helpers do NOT flip this — only
    /// deliberate use of the upstream session counts.
    ///
    /// Stored atomically because the setters live behind `&self`-only
    /// call paths; `Ordering::Relaxed` is sufficient (we never
    /// synchronize anything else against this load/store).
    ///
    /// Also flipped to `true` at the top of [`super::Connection::delete`]
    /// so an explicit teardown of a fresh-mint connection can't race
    /// the drop-time orphan-DELETE into a double-fire.
    any_calls: AtomicBool,

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
    /// drops, so existing `Connection::Drop` semantics are unchanged.
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
    /// Creates a minimal connection for unit testing. Declares both
    /// `tools` and `resources` capabilities with `list_changed:
    /// Some(true)` so callers exercise the present-cap +
    /// list_changed-enabled paths in `list_*`, `refresh_*`, and
    /// `subscribe_*`. For other capability shapes use
    /// `new_for_test_with_caps`.
    #[cfg(test)]
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
    #[cfg(test)]
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
            call_timeout: Duration::from_secs(30),
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
            is_reconnect: false,
            any_calls: AtomicBool::new(false),
            next_id: AtomicU64::new(2),
            // Test connection has no listener and never refreshes; seed
            // with an empty Ok so `list_tools` doesn't try to paginate.
            tools: RwLock::new(Some(Ok(Arc::new(Vec::new())))),
            resources: RwLock::new(Some(Ok(Arc::new(Vec::new())))),
            _listener_cancel_guard: std::sync::Mutex::new(None),
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
        is_reconnect: bool,
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
            is_reconnect,
            any_calls: AtomicBool::new(false),
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

    /// Server declared `tools.list_changed: true`. Gates
    /// `subscribe_tools`'s wait-for-notify branch: when `false` the
    /// upstream will never push `notifications/tools/list_changed`, so
    /// awaiting the notify is unreachable and `subscribe_tools` returns
    /// the current cache immediately.
    fn has_tools_list_changed(&self) -> bool {
        matches!(
            self.initialize_result.capabilities.tools,
            Some(super::initialize_result::ToolsCapability {
                list_changed: Some(true),
            })
        )
    }

    /// Server declared `resources.list_changed: true`. Symmetric to
    /// `has_tools_list_changed`; gates `subscribe_resources`'s
    /// wait-for-notify branch.
    fn has_resources_list_changed(&self) -> bool {
        matches!(
            self.initialize_result.capabilities.resources,
            Some(super::initialize_result::ResourcesCapability {
                list_changed: Some(true),
                ..
            })
        )
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
    async fn post(&self) -> reqwest::RequestBuilder {
        self.http_client
            .post(&self.url)
            .timeout(self.call_timeout)
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

    /// Sends a JSON-RPC notification (no response expected) with the
    /// same exponential-backoff retry policy as [`Self::rpc`]. Every
    /// error is transient; the loop gives up only when the backoff's
    /// `max_elapsed_time` is exceeded.
    async fn notify<P: serde::Serialize>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<(), super::Error> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        backoff::future::retry(self.backoff(), || async {
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
        let url = format!("{}/notify", self.url.trim_end_matches('/'));
        let request = self
            .http_client
            .get(&url)
            .timeout(self.call_timeout)
            .headers(
                self.build_request_headers(None, Some("application/json"))
                    .await,
            );

        let response =
            request
                .send()
                .await
                .map_err(|source| super::Error::Request {
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

    /// `POST <self.url>/notify` against the ObjectiveAI MCP proxy.
    /// Appends `blocks` to the proxy's pending-notifications queue for
    /// this session. Single-attempt — the caller decides whether to
    /// retry. A 404 (session unknown) surfaces as `SessionExpired`
    /// rather than `Ok(())` because the caller is asking for delivery
    /// and a lost session means delivery did not happen.
    async fn enqueue_notifications(
        &self,
        blocks: &[super::tool::ContentBlock],
    ) -> Result<(), super::Error> {
        let url = format!("{}/notify", self.url.trim_end_matches('/'));
        let request = self
            .http_client
            .post(&url)
            .timeout(self.call_timeout)
            .headers(
                self.build_request_headers(
                    Some("application/json"),
                    Some("application/json"),
                )
                .await,
            )
            .json(blocks);

        let response =
            request
                .send()
                .await
                .map_err(|source| super::Error::Request {
                    url: url.clone(),
                    source,
                })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(super::Error::SessionExpired { url });
        }
        if !response.status().is_success() {
            let code = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(super::Error::BadStatus { url, code, body });
        }

        Ok(())
    }

    /// `GET <self.url>/notify/queued` against the ObjectiveAI MCP proxy.
    /// Non-draining peek — returns `true` iff the proxy's
    /// pending-notifications queue for this session is non-empty.
    /// A 404 (session unknown) is mapped to `Ok(false)` to match the
    /// drain path's soft-fallback contract.
    async fn has_pending_notifications(&self) -> Result<bool, super::Error> {
        let url = format!("{}/notify/queued", self.url.trim_end_matches('/'));
        let request = self
            .http_client
            .get(&url)
            .timeout(self.call_timeout)
            .headers(
                self.build_request_headers(None, Some("application/json"))
                    .await,
            );

        let response =
            request
                .send()
                .await
                .map_err(|source| super::Error::Request {
                    url: url.clone(),
                    source,
                })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(false);
        }
        if !response.status().is_success() {
            let code = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(super::Error::BadStatus { url, code, body });
        }

        response
            .json::<bool>()
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
        // Mark the connection as deliberately used. Flipped at the top
        // of the method (not after success) because even a failed
        // `tools/call` may have mutated upstream state — we don't want
        // the drop-time orphan-DELETE second-guessing that.
        self.any_calls.store(true, Ordering::Relaxed);
        let mut result: super::tool::CallToolResult =
            self.rpc("tools/call", params, false).await?;

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
        if !self.has_tools_list_changed() {
            // Server can't push `notifications/tools/list_changed` —
            // awaiting the notify is unreachable. Return whatever's
            // there right now (`Ok(empty)` on a tools-cap-absent
            // server via `list_tools`'s own gate; the real cache
            // otherwise).
            return self.list_tools().await;
        }
        // Arm BEFORE reading. `enable()` registers the future in the
        // wait queue without polling, so a `notify_waiters` racing
        // between our read and our await still wakes us.
        let notified = self.tools_changed.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();

        // `list_tools` handles a cleared cache (post-drop) by
        // paginating inline. A `None` initial state can't be
        // compared to the caller's snapshot meaningfully — promote
        // to whatever the refresh installs.
        let initial = self.list_tools().await;
        match &initial {
            Ok(arc) if arc.as_slice() == current => {}
            _ => return initial,
        }

        let _ = tokio::time::timeout(timeout, notified).await;

        self.list_tools().await
    }

    /// Resource counterpart of [`Self::subscribe_tools`].
    async fn subscribe_resources(
        &self,
        current: &[super::resource::Resource],
        timeout: Duration,
    ) -> Result<Arc<Vec<super::resource::Resource>>, Arc<super::Error>> {
        if !self.has_resources_list_changed() {
            // Symmetric to `subscribe_tools` — see that gate.
            return self.list_resources().await;
        }
        let notified = self.resources_changed.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();

        let initial = self.list_resources().await;
        match &initial {
            Ok(arc) if arc.as_slice() == current => {}
            _ => return initial,
        }

        let _ = tokio::time::timeout(timeout, notified).await;

        self.list_resources().await
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
        // Mark the connection as deliberately used (same reasoning as
        // `call_tool` — see the drop-time orphan-DELETE gate).
        self.any_calls.store(true, Ordering::Relaxed);
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
        // `notify_waiters` and `on_change` fire under the write
        // guard, *after* `*guard = result`, so anyone awoken by them
        // queues on the read lock, waits for the guard to drop, and
        // observes the post-swap state.
        let (mut guard, result) =
            tokio::join!(self.tools.write(), self.paginate_tools(),);
        *guard = Some(result);
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

    /// Resource counterpart of [`Self::refresh_tools_signaling`]. The
    /// same spawn-site-gate invariant applies: the caller must gate
    /// the spawn on `capabilities.resources.is_some()`.
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
        *guard = Some(result);
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
                            this.get().await.send().await
                        } => out,
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
                    this.refresh_resources(
                        this.on_resources_list_changed.get()
                    ),
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

impl Drop for ConnectionInner {
    /// Orphan-DELETE hook: when a **freshly-minted** connection (not a
    /// resume) is dropped without any deliberate use — no `call_tool`,
    /// no `read_resource`, no explicit `Connection::delete` — spawn a
    /// fire-and-forget HTTP DELETE so the upstream session we just
    /// opened doesn't sit there accruing per-session state for nothing.
    ///
    /// Reconnect-resumes are deliberately excluded: a reconnect's
    /// `any_calls == false` only means *this* `Connection` instance
    /// never called anything — the underlying upstream session may
    /// well have been used by an earlier `Connection` that opened it,
    /// did real work, and let us re-attach. Tearing it down here would
    /// kill a still-live session out from under whoever owns it.
    /// Reconnects rely on the proxy's explicit `Connection::delete`
    /// (or `Client::delete`) for upstream cleanup instead.
    ///
    /// Skip conditions (any one triggers a no-op):
    ///
    /// - `mock` is `true` — there was never an HTTP session to begin with.
    /// - `is_reconnect` is `true` — see above; the upstream session
    ///   pre-existed this connection and isn't ours to tear down on drop.
    /// - `any_calls` is `true` — the connection was deliberately used,
    ///   or an explicit `Connection::delete` already ran.
    /// - No tokio runtime is in scope — `tokio::spawn` would panic.
    ///   Silently leak the upstream session in this case (sync
    ///   teardown paths e.g. `cfg(test)` blocks not driven by tokio).
    ///
    /// The listener-cancel `DropGuard` inside `_listener_cancel_guard`
    /// fires automatically as part of this same `drop` call, so by the
    /// time the orphan DELETE goes out the listener task has already
    /// been told to cancel — no SSE/GET race with the upstream DELETE.
    fn drop(&mut self) {
        if self.is_reconnect {
            return;
        }
        if self.any_calls.load(Ordering::Relaxed) {
            return;
        }

        // Clone out the bits the orphan task needs. None of these are
        // big: `reqwest::Client` is itself an `Arc` bump,
        // `IndexMap<String, String>` is the per-connection header bag
        // (small), and the `String`s are the per-session id + URL.
        let http_client = self.http_client.clone();
        let url = self.url.clone();
        let session_id = self.session_id.clone();
        let headers = self.headers.clone();
        let timeout = self.call_timeout;

        // Spawn only if a tokio runtime is in scope. `tokio::spawn`
        // panics outside one — sync teardown paths (e.g. a test that
        // builds a `Connection` and lets it drop on a non-async stack)
        // would crash without this guard.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(orphan_delete(
                http_client,
                url,
                session_id,
                headers,
                timeout,
            ));
        }
    }
}

/// Fire-and-forget HTTP `DELETE` used by [`ConnectionInner::drop`] to
/// release a resumed upstream session that was never used. Mirrors
/// [`super::Connection::delete`]'s wire shape (same `Mcp-Session-Id`
/// header behavior, same header-loop with the explicit session id
/// winning over any `Mcp-Session-Id` entry in `headers`) but never
/// surfaces errors — there's no caller left to surface them to. The
/// `timeout` (sourced from the originating connection's `call_timeout`)
/// caps the request so a hanging upstream can't keep the spawned task
/// alive forever.
async fn orphan_delete(
    http_client: reqwest::Client,
    url: String,
    session_id: String,
    headers: IndexMap<String, String>,
    timeout: Duration,
) {
    let mut request = http_client
        .delete(&url)
        .timeout(timeout)
        .header("Mcp-Session-Id", &session_id);
    for (name, value) in &headers {
        if name.eq_ignore_ascii_case("Mcp-Session-Id") {
            continue;
        }
        request = request.header(name, value);
    }
    // Errors silently swallowed: no caller, and the listener-cancel
    // guard has already fired (it runs as part of the regular
    // `_listener_cancel_guard` drop inside the same `drop` call).
    let _ = request.send().await;
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

    /// 3.7 — `subscribe_tools` returns immediately (no wait-for-notify)
    /// when the server declared `tools` but not
    /// `tools.list_changed: Some(true)`. We populate the cache with a
    /// non-empty list, pass a long timeout, and expect a fast return
    /// with the cache contents.
    #[tokio::test]
    async fn subscribe_tools_short_circuits_when_list_changed_absent() {
        let conn = Connection::new_for_test_with_caps(
            "t".into(),
            "http://x".into(),
            caps(
                Some(ToolsCapability { list_changed: None }),
                None,
            ),
        );
        *conn.inner.tools.write().await =
            Some(Ok(Arc::new(vec![tool("a")])));

        let start = std::time::Instant::now();
        let got = conn
            .subscribe_tools(&[tool("a")], Duration::from_secs(5))
            .await
            .unwrap();
        assert!(
            start.elapsed() < Duration::from_millis(100),
            "elapsed: {:?}",
            start.elapsed()
        );
        assert_eq!(got.as_slice(), &[tool("a")]);
    }

    /// 3.8 — symmetric to 3.7 for `subscribe_resources`.
    #[tokio::test]
    async fn subscribe_resources_short_circuits_when_list_changed_absent() {
        use crate::mcp::resource::Resource;
        let conn = Connection::new_for_test_with_caps(
            "t".into(),
            "http://x".into(),
            caps(
                None,
                Some(ResourcesCapability {
                    subscribe: None,
                    list_changed: None,
                }),
            ),
        );
        let res = Resource {
            uri: "file://a".into(),
            name: "a".into(),
            title: None,
            description: None,
            mime_type: None,
            annotations: None,
            icons: None,
            _meta: None,
        };
        *conn.inner.resources.write().await =
            Some(Ok(Arc::new(vec![res.clone()])));

        let start = std::time::Instant::now();
        let got = conn
            .subscribe_resources(&[res.clone()], Duration::from_secs(5))
            .await
            .unwrap();
        assert!(
            start.elapsed() < Duration::from_millis(100),
            "elapsed: {:?}",
            start.elapsed()
        );
        assert_eq!(got.as_slice(), &[res]);
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
        *conn.inner.tools.write().await = Some(Ok(Arc::new(vec![tool("a")])));

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
        *conn.inner.tools.write().await = Some(Err(Arc::new(err)));

        let start = std::time::Instant::now();
        let got = conn.subscribe_tools(&[], Duration::from_secs(5)).await;
        assert!(start.elapsed() < Duration::from_millis(100));
        assert!(got.is_err());
    }

    /// Cache equals snapshot, then a writer fires the notify under the
    /// write lock and installs a new list. The subscriber wakes, then its
    /// re-read blocks behind the writer's guard, observes the new list.
    #[tokio::test]
    async fn subscribe_tools_wakes_on_change_and_reads_post_swap() {
        let conn = Connection::new_for_test("t".into(), "http://x".into());
        *conn.inner.tools.write().await = Some(Ok(Arc::new(vec![tool("a")])));

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
            *guard = Some(Ok(Arc::new(vec![tool("b")])));
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
        *conn.inner.tools.write().await = Some(Ok(Arc::new(vec![tool("a")])));

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
        *conn.inner.tools.write().await = Some(Ok(Arc::new(vec![tool("a")])));

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
            *guard = Some(Ok(Arc::new(vec![tool("c")])));
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
            ContentBlock::Text(TextContent { text, .. }) => {
                assert_eq!(text, "first")
            }
            other => panic!("expected text, got {other:?}"),
        }
        match &blocks[1] {
            ContentBlock::Text(TextContent { text, .. }) => {
                assert_eq!(text, "second")
            }
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
        let err = conn.drain_notifications().await.expect_err("5xx → err");
        match err {
            super::super::Error::BadStatus { code, body, .. } => {
                assert_eq!(code.as_u16(), 500);
                assert_eq!(body, "boom");
            }
            other => panic!("expected BadStatus, got {other:?}"),
        }
    }

}

#[cfg(test)]
mod has_pending_notifications_tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Happy path: proxy returns `true` → Ok(true).
    #[tokio::test]
    async fn has_pending_notifications_true() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notify/queued"))
            .and(header("Mcp-Session-Id", ""))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(true)))
            .mount(&server)
            .await;

        let conn = Connection::new_for_test("t".into(), server.uri());
        let got = conn.has_pending_notifications().await.expect("peek ok");
        assert!(got);
    }

    /// Proxy returns `false` → Ok(false).
    #[tokio::test]
    async fn has_pending_notifications_false() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notify/queued"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!(false)),
            )
            .mount(&server)
            .await;

        let conn = Connection::new_for_test("t".into(), server.uri());
        let got = conn.has_pending_notifications().await.expect("peek ok");
        assert!(!got);
    }

    /// 404 (proxy lost the session) → Ok(false). Same soft-fallback
    /// contract as drain_notifications — peek must never abort a
    /// request over a missing-session race.
    #[tokio::test]
    async fn has_pending_notifications_404_returns_false() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notify/queued"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let conn = Connection::new_for_test("t".into(), server.uri());
        let got = conn
            .has_pending_notifications()
            .await
            .expect("404 → ok(false)");
        assert!(!got);
    }

    /// Non-success / non-404 status propagates as `BadStatus`.
    #[tokio::test]
    async fn has_pending_notifications_5xx_returns_bad_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notify/queued"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let conn = Connection::new_for_test("t".into(), server.uri());
        let err = conn
            .has_pending_notifications()
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

}
