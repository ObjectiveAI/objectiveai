//! MCP connection for communicating with an MCP server.
//!
//! [`Connection`] is a cheaply-clonable handle around an internal
//! [`ConnectionInner`]. Cloning bumps the inner refcount; dropping fires
//! `external_dropped.notify_waiters()` so the long-lived background SSE
//! listener wakes up and can re-check liveness immediately, exiting once
//! it observes that no external `Connection` handle remains.

use std::ops::Deref;
use std::sync::{Arc, RwLock as StdRwLock, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use indexmap::IndexMap;
use tokio::sync::{Notify, RwLock};

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
/// Cheaply clonable (one `Arc` bump). Dropping any handle fires the
/// internal `external_dropped` `Notify` so the upstream SSE listener task
/// wakes immediately and can re-check `Arc::strong_count(&inner)`. When
/// the listener sees only itself holding the inner Arc, it exits and the
/// upstream HTTP session closes.
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

impl Drop for Connection {
    fn drop(&mut self) {
        // Wake the upstream SSE listener so it can re-check liveness
        // without waiting for the upstream's keepalive interval. Cheap:
        // just notifies wakers, returns immediately.
        self.inner.external_dropped.notify_waiters();
    }
}

impl Deref for Connection {
    type Target = ConnectionInner;
    fn deref(&self) -> &ConnectionInner {
        &self.inner
    }
}

impl Connection {
    pub(super) fn new(
        http_client: reqwest::Client,
        url: String,
        session_id: String,
        authorization: Option<String>,
        user_agent: String,
        x_title: String,
        http_referer: String,
        extra_headers: IndexMap<String, String>,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
        call_timeout: Duration,
        initialize_result: super::initialize_result::InitializeResult,
    ) -> Self {
        let inner = ConnectionInner::new(
            http_client,
            url,
            session_id,
            authorization,
            user_agent,
            x_title,
            http_referer,
            extra_headers,
            backoff_current_interval,
            backoff_initial_interval,
            backoff_randomization_factor,
            backoff_multiplier,
            backoff_max_interval,
            backoff_max_elapsed_time,
            call_timeout,
            initialize_result,
        );
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
    pub authorization: Option<String>,
    pub user_agent: String,
    pub x_title: String,
    pub http_referer: String,
    /// Extra HTTP headers forwarded on every POST and GET this connection
    /// makes. Applied *after* the fixed headers above (`Content-Type`,
    /// `Mcp-Session-Id`, `User-Agent`, etc.) so the fixed set always wins.
    pub extra_headers: IndexMap<String, String>,

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

    /// Wakeup signal for the long-lived `listen_for_list_changes` task.
    /// Fired by [`Connection`]'s `Drop` impl on every external handle drop
    /// so the listener can re-check `Arc::strong_count` immediately and
    /// exit when no external handle remains.
    external_dropped: Arc<Notify>,

    /// Optional callback fired *after* the listener has refreshed the
    /// tool cache in response to an upstream `notifications/tools/list_changed`.
    /// Set via [`Connection::set_on_tools_list_changed`].
    on_tools_list_changed: CallbackSlot,

    /// Optional callback fired *after* the listener has refreshed the
    /// resource cache in response to an upstream
    /// `notifications/resources/list_changed`.
    /// Set via [`Connection::set_on_resources_list_changed`].
    on_resources_list_changed: CallbackSlot,
}

impl ConnectionInner {
    /// Creates a mock connection that never makes network requests.
    /// All RPC calls return empty/default results.
    fn new_mock(url: String) -> Arc<Self> {
        Arc::new(Self {
            http_client: reqwest::Client::new(),
            url,
            session_id: String::new(),
            authorization: None,
            user_agent: String::new(),
            x_title: String::new(),
            http_referer: String::new(),
            extra_headers: IndexMap::new(),
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
            external_dropped: Arc::new(Notify::new()),
            on_tools_list_changed: CallbackSlot::new(),
            on_resources_list_changed: CallbackSlot::new(),
        })
    }

    /// Creates a minimal connection for unit testing.
    #[cfg(test)]
    fn new_for_test(name: String, url: String) -> Arc<Self> {
        Arc::new(Self {
            http_client: reqwest::Client::new(),
            url,
            session_id: String::new(),
            authorization: None,
            user_agent: String::new(),
            x_title: String::new(),
            http_referer: String::new(),
            extra_headers: IndexMap::new(),
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
            external_dropped: Arc::new(Notify::new()),
            on_tools_list_changed: CallbackSlot::new(),
            on_resources_list_changed: CallbackSlot::new(),
        })
    }

    /// Creates a new connection and spawns background tasks to paginate
    /// all tools and resources. Called internally by
    /// [`Client::connect`](super::Client::connect) (via [`Connection::new`]).
    fn new(
        http_client: reqwest::Client,
        url: String,
        session_id: String,
        authorization: Option<String>,
        user_agent: String,
        x_title: String,
        http_referer: String,
        extra_headers: IndexMap<String, String>,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
        call_timeout: Duration,
        initialize_result: super::initialize_result::InitializeResult,
    ) -> Arc<Self> {
        let conn = Arc::new(Self {
            http_client,
            url,
            session_id,
            authorization,
            user_agent,
            x_title,
            http_referer,
            extra_headers,
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
            external_dropped: Arc::new(Notify::new()),
            on_tools_list_changed: CallbackSlot::new(),
            on_resources_list_changed: CallbackSlot::new(),
        });

        // Spawn background tool lister if the server supports tools.
        // Initial population happens before any callback could be
        // registered, so pass `None` — there's no list-change to signal
        // for the very first fetch.
        if conn.initialize_result.capabilities.tools.is_some() {
            let conn = Arc::clone(&conn);
            tokio::spawn(async move {
                conn.refresh_tools(None).await;
            });
        }

        // Spawn background resource lister if the server supports resources.
        if conn.initialize_result.capabilities.resources.is_some() {
            let conn = Arc::clone(&conn);
            tokio::spawn(async move {
                conn.refresh_resources(None).await;
            });
        }

        // Spawn listener for list_changed notifications if supported.
        {
            let tools_list_changed = conn
                .initialize_result
                .capabilities
                .tools
                .and_then(|t| t.list_changed)
                .unwrap_or(false);
            let resources_list_changed = conn
                .initialize_result
                .capabilities
                .resources
                .and_then(|r| r.list_changed)
                .unwrap_or(false);

            if tools_list_changed || resources_list_changed {
                // Hand the listener a `Weak` so it can self-cancel once
                // every external strong reference to this Connection is
                // dropped. If we cloned an `Arc` instead, the spawned task
                // would itself keep the Connection alive forever.
                //
                // Also clone the `external_dropped` Notify so the listener
                // wakes up *immediately* when an external wrapper around
                // `Arc<Connection>` is dropped — see
                // `listen_for_list_changes` doc.
                let weak = Arc::downgrade(&conn);
                let external_dropped = Arc::clone(&conn.external_dropped);
                tokio::spawn(async move {
                    Self::listen_for_list_changes(
                        weak,
                        external_dropped,
                        tools_list_changed,
                        resources_list_changed,
                    )
                    .await;
                });
            }
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
            .header("Accept", "application/json, text/event-stream")
            .header("Mcp-Session-Id", &self.session_id);

        if let Some(auth) = &self.authorization {
            request = request.header("Authorization", auth);
        }
        request = request.header("User-Agent", &self.user_agent);
        request = request.header("X-Title", &self.x_title);
        request = request.header("Referer", &self.http_referer);
        request = request.header("HTTP-Referer", &self.http_referer);
        for (name, value) in &self.extra_headers {
            request = request.header(name, value);
        }
        request
    }

    /// Sends a JSON-RPC request with exponential backoff retries.
    ///
    /// Network errors and non-success HTTP status codes are retried.
    /// Session expiration (404) and JSON-RPC errors are permanent failures.
    async fn rpc<P: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<R, super::Error> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        backoff::future::retry(self.backoff(), || async {
            let response =
                self.post().json(&body).send().await.map_err(|e| {
                    backoff::Error::transient(super::Error::Request(e))
                })?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(backoff::Error::permanent(
                    super::Error::SessionExpired,
                ));
            }
            if !response.status().is_success() {
                let code = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(backoff::Error::transient(
                    super::Error::BadStatus { code, body },
                ));
            }

            let rpc_response: super::JsonRpcResponse<R> =
                response.json().await.map_err(|e| {
                    backoff::Error::transient(super::Error::Request(e))
                })?;

            match rpc_response {
                super::JsonRpcResponse::Success { result, .. } => Ok(result),
                super::JsonRpcResponse::Error { error, .. } => {
                    Err(backoff::Error::permanent(super::Error::JsonRpc {
                        code: error.code,
                        message: error.message,
                        data: error.data,
                    }))
                }
            }
        })
        .await
    }

    /// Sends a JSON-RPC notification (no response expected, no retries).
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

        let response = self
            .post()
            .json(&body)
            .send()
            .await
            .map_err(super::Error::Request)?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(super::Error::SessionExpired);
        }
        if !response.status().is_success() {
            let code = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(super::Error::BadStatus { code, body });
        }

        Ok(())
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
        self.rpc("tools/call", params).await
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
        let mut guard = self.tools.write().await;
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
        let mut guard = self.resources.write().await;
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
            .header("Accept", "text/event-stream")
            .header("Mcp-Session-Id", &self.session_id);

        if let Some(auth) = &self.authorization {
            request = request.header("Authorization", auth);
        }
        request = request.header("User-Agent", &self.user_agent);
        request = request.header("X-Title", &self.x_title);
        request = request.header("Referer", &self.http_referer);
        request = request.header("HTTP-Referer", &self.http_referer);
        for (name, value) in &self.extra_headers {
            request = request.header(name, value);
        }
        request
    }

    /// Opens a GET SSE stream to the MCP endpoint and listens for
    /// `notifications/tools/list_changed` and
    /// `notifications/resources/list_changed`. On each notification,
    /// write-locks and re-fetches the full list. Reconnects on
    /// disconnection with a brief delay.
    ///
    /// Takes a [`Weak<Self>`] (not `Arc<Self>`) so the spawned task
    /// doesn't itself keep the [`Connection`] alive.
    ///
    /// Cancellation is event-driven, not poll-based:
    ///
    /// 1. At the top of every outer-loop iteration we upgrade the weak;
    ///    if every external strong reference is gone, the task returns.
    /// 2. While parked inside the SSE read loop, we [`tokio::select!`]
    ///    the line reader against `external_dropped.notified()`. External
    ///    wrappers around `Arc<Connection>` fire `notify_waiters()` from
    ///    their `Drop` impl; when that fires, we re-check
    ///    `Arc::strong_count(&this) == 1` and exit immediately if no
    ///    external holder remains.
    /// 3. As a backup for the race where a drop fires *between* iterations
    ///    (we missed the notify because we weren't yet registered), the
    ///    top of every inner-loop iteration also checks the strong count.
    async fn listen_for_list_changes(
        weak: Weak<Self>,
        external_dropped: Arc<Notify>,
        tools: bool,
        resources: bool,
    ) {
        use futures_util::TryStreamExt;
        use tokio::io::AsyncBufReadExt;
        use tokio_util::io::StreamReader;

        loop {
            // Layer 1: between-reconnect liveness check.
            let Some(this) = weak.upgrade() else { return };
            let backoff_delay = this.backoff_initial_interval;

            let response = match this.get().send().await {
                Ok(r) if r.status().is_success() => r,
                _ => {
                    drop(this);
                    tokio::time::sleep(backoff_delay).await;
                    continue;
                }
            };

            let stream = response
                .bytes_stream()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
            let reader = StreamReader::new(stream);
            let mut lines = reader.lines();

            'inner: loop {
                // Layer 3: race-window backup. If a drop fired between
                // our last `notified()` registration and this one, the
                // notify is gone — but the strong count tells us.
                if Arc::strong_count(&this) == 1 {
                    return;
                }

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
                                    "notifications/tools/list_changed" if tools => {
                                        // refresh_tools fires the
                                        // callback after taking the
                                        // write lock but before the
                                        // network paginate, so the
                                        // proxy's downstream
                                        // notifications/tools/list_changed
                                        // emission lines up with the
                                        // staleness window opening.
                                        this.refresh_tools(
                                            this.on_tools_list_changed.get(),
                                        )
                                        .await;
                                    }
                                    "notifications/resources/list_changed" if resources => {
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
                    // Layer 2: an external wrapper just fired notify_waiters
                    // from its Drop. Loop back; the strong-count check at
                    // the top decides whether to exit.
                    _ = external_dropped.notified() => {}
                }
            }

            // Stream ended — drop the strong ref before sleeping so the
            // next iteration's weak-upgrade can detect liveness honestly.
            drop(this);
            tokio::time::sleep(backoff_delay).await;
        }
    }
}

