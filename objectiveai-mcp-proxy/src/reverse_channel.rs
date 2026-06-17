//! WS reverse-channel transport for CLI-hosted upstreams.
//!
//! When the proxy is embedded in the API (per request), it is handed a
//! [`ReverseChannel`] — the means to speak the `client_objectiveai_mcp`
//! protocol over the request's WebSocket. Upstreams whose URL scheme is
//! `ws` ([`WsUpstream`]) are reached through it instead of over HTTP:
//!
//! - `ws://objectiveai` → [`McpKind::ObjectiveAi`]
//! - `ws:///owner/name/version/mcp` → [`McpKind::Other`]
//!
//! Direction split (the API owns the WS itself):
//! - **send**: the proxy emits a `server_request::Request` into the
//!   channel's mpsc; the API serializes it onto the shared WS sink.
//! - **recv**: the API's recv loop demuxes incoming frames by type and
//!   hands the proxy-bound ones back via [`ReverseChannel::deliver_response`]
//!   (the 6 MCP `server_response` variants) and
//!   [`ReverseChannel::deliver_client_request`] (`McpListChanged`). The
//!   proxy correlates responses to its own outstanding requests by id.
//!
//! [`Upstream`] is the proxy's per-upstream handle — either an HTTP
//! [`Connection`] or a [`WsUpstream`] — exposing the slice of the
//! `Connection` interface the [`crate::session::Session`] depends on.

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::{
    McpKind,
    client_request::{self, McpListChangedKind},
    client_response,
    server_request::{self, InitializeRequest, Request as ServerRequest},
    server_response::{self, JsonRpcResult, Response as ServerResponse},
};
use objectiveai_sdk::mcp::resource::{
    ListResourcesRequest, ReadResourceRequestParams, ReadResourceResult, Resource,
};
use objectiveai_sdk::mcp::tool::{
    CallToolRequestParams, CallToolResult, ListToolsRequest, Tool,
};
use objectiveai_sdk::mcp::{Connection, Error as McpError};
use tokio::sync::{RwLock, mpsc, oneshot};

/// A list-changed callback (mirrors `Connection::set_on_*_list_changed`).
type ListChangedCb = Arc<dyn Fn() + Send + Sync>;

struct Inner {
    /// proxy → API → WS. The API drains the paired receiver and writes
    /// each request onto the shared WS sink.
    tx: mpsc::UnboundedSender<ServerRequest>,
    /// Outstanding requests awaiting their `server_response`, by id.
    pending: DashMap<String, oneshot::Sender<ServerResponse>>,
    /// Per-upstream round-trip budget.
    timeout: Duration,
    /// list-changed callbacks per upstream `McpKind`: `(tools, resources)`.
    /// Fired when a matching `client_request::McpListChanged` arrives.
    list_changed: DashMap<McpKind, (Option<ListChangedCb>, Option<ListChangedCb>)>,
}

/// Cheaply-cloneable handle the proxy uses to speak over the WS.
#[derive(Clone)]
pub struct ReverseChannel(Arc<Inner>);

impl std::fmt::Debug for ReverseChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReverseChannel").finish_non_exhaustive()
    }
}

impl ReverseChannel {
    /// Build a channel. Returns the channel plus the receiver the API
    /// drains (serializing each `server_request` onto the shared WS sink).
    pub fn new(timeout: Duration) -> (Self, mpsc::UnboundedReceiver<ServerRequest>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let inner = Inner {
            tx,
            pending: DashMap::new(),
            timeout,
            list_changed: DashMap::new(),
        };
        (Self(Arc::new(inner)), rx)
    }

    /// Emit a `server_request` and await its matching `server_response`,
    /// bounded by the configured timeout. `id` is minted here; the API's
    /// recv loop routes the reply back via [`Self::deliver_response`].
    async fn request(
        &self,
        payload: server_request::Payload,
        headers: IndexMap<String, String>,
    ) -> Result<ServerResponse, McpError> {
        let id = uuid::Uuid::new_v4().to_string();
        let (resp_tx, resp_rx) = oneshot::channel();
        self.0.pending.insert(id.clone(), resp_tx);
        let request = ServerRequest {
            id: id.clone(),
            headers,
            payload,
        };
        if self.0.tx.send(request).is_err() {
            self.0.pending.remove(&id);
            return Err(transport_error("reverse channel closed before send"));
        }
        match tokio::time::timeout(self.0.timeout, resp_rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                self.0.pending.remove(&id);
                Err(transport_error("reverse channel dropped before response"))
            }
            Err(_) => {
                self.0.pending.remove(&id);
                Err(transport_error("reverse channel timed out waiting for response"))
            }
        }
    }

    /// Hand a proxy-bound `server_response` (one of the 6 MCP variants)
    /// back to the waiter that issued the matching request. Called by the
    /// API's recv loop. Unknown id → dropped.
    pub fn deliver_response(&self, response: ServerResponse) {
        if let Some((_, tx)) = self.0.pending.remove(&response.id) {
            let _ = tx.send(response);
        }
    }

    /// Hand a proxy-bound `client_request` (today only `McpListChanged`)
    /// to the proxy. Fires the registered list-changed callback for the
    /// upstream, and returns the ack the API writes back over the WS.
    pub fn deliver_client_request(
        &self,
        request: client_request::Request,
    ) -> client_response::Response {
        let client_request::Request { id, payload } = request;
        match payload {
            client_request::Payload::McpListChanged(change) => {
                if let Some(cbs) = self.0.list_changed.get(&change.mcp_kind) {
                    let cb = match change.kind {
                        McpListChangedKind::Tools => cbs.0.clone(),
                        McpListChangedKind::Resources => cbs.1.clone(),
                    };
                    drop(cbs);
                    if let Some(cb) = cb {
                        cb();
                    }
                }
                client_response::Response::Ok { id }
            }
        }
    }

    fn set_tools_list_changed(&self, mcp_kind: McpKind, cb: ListChangedCb) {
        let mut entry = self.0.list_changed.entry(mcp_kind).or_default();
        entry.0 = Some(cb);
    }

    fn set_resources_list_changed(&self, mcp_kind: McpKind, cb: ListChangedCb) {
        let mut entry = self.0.list_changed.entry(mcp_kind).or_default();
        entry.1 = Some(cb);
    }
}

/// A `ws://`-scheme upstream, reached over the [`ReverseChannel`]. Mirrors
/// the slice of [`Connection`]'s interface the [`crate::session::Session`]
/// uses, translating each op into a `server_request` carrying this
/// upstream's [`McpKind`].
pub struct WsUpstream {
    channel: ReverseChannel,
    mcp_kind: McpKind,
    /// The `ws://…` URL this upstream was dialed with (used for filtering).
    pub url: String,
    /// Upstream `Mcp-Session-Id` returned by the CLI on `initialize`.
    pub session_id: String,
    /// Upstream `server_info.name` / `.version` from the `initialize`
    /// reply — feeds the session's routing-prefix derivation.
    server_name: String,
    server_version: String,
    /// Whether the upstream advertised the `tools` / `resources`
    /// capability in its `initialize` reply. We must NOT issue
    /// `tools/list` / `resources/list` against an upstream that didn't
    /// advertise the capability: many servers (incl. the test
    /// fixtures) 404 the un-advertised method, and a hard error there
    /// fails the whole aggregate — and, on the post-init health probe,
    /// fails the connect and churns endless re-`initialize`s. Mirrors
    /// `mcp::Connection::has_{tools,resources}_cap`.
    has_tools_cap: bool,
    has_resources_cap: bool,
    /// Persistent per-upstream headers captured at connect: the per-URL
    /// set (`Authorization`, custom `X-*`, `X-OBJECTIVEAI-ARGUMENTS`)
    /// plus whatever identity headers were present at dial. Never
    /// mutated after connect — mirrors the SDK `Connection`'s base
    /// `headers`. The transient subset is overridden per request by
    /// `extra_headers`; the per-URL subset has no overlay key, so it
    /// always survives on every request.
    base_headers: IndexMap<String, String>,
    /// Mutable transient-identity overlay, full-replaced every turn by
    /// `apply_transient_headers` → `set_extra_headers`. Overrides
    /// `base_headers` per key (mirrors `Connection::extra_headers`).
    /// Starts empty: until the first refresh, `base_headers` alone
    /// carries the dial-time identity headers.
    extra_headers: RwLock<IndexMap<String, String>>,
}

impl std::fmt::Debug for WsUpstream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsUpstream")
            .field("url", &self.url)
            .field("session_id", &self.session_id)
            .finish_non_exhaustive()
    }
}

impl WsUpstream {
    /// Headers to stamp on every outbound `server_request`, mirroring
    /// `Connection::build_request_headers` exactly: the persistent
    /// `base_headers` first, then the `extra_headers` transient overlay
    /// (overrides per key), then this upstream's `Mcp-Session-Id` last
    /// (so it can never be shadowed). Per-URL headers live only in
    /// `base_headers` (no overlay key collides), so they're present on
    /// EVERY request — identical to the HTTP path.
    async fn headers(&self) -> IndexMap<String, String> {
        let mut h = self.base_headers.clone();
        for (k, v) in self.extra_headers.read().await.iter() {
            h.insert(k.clone(), v.clone());
        }
        h.insert(
            crate::upstream::MCP_SESSION_ID_KEY.to_string(),
            self.session_id.clone(),
        );
        h
    }

    pub async fn list_tools(&self) -> Result<Arc<Vec<Tool>>, Arc<McpError>> {
        // Capability gate — an upstream that didn't advertise `tools`
        // has no `tools/list`; calling it anyway 404s on most servers.
        if !self.has_tools_cap {
            return Ok(Arc::new(Vec::new()));
        }
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::ToolsList {
                    mcp_kind: self.mcp_kind.clone(),
                    params: ListToolsRequest { cursor: None },
                },
                headers,
            )
            .await
            .map_err(Arc::new)?;
        match response.payload {
            server_response::Payload::ToolsList { result, .. } => {
                Ok(Arc::new(unwrap_rpc(&self.url, result).map_err(Arc::new)?.tools))
            }
            other => Err(Arc::new(variant_mismatch(&self.url, "tools_list", &other))),
        }
    }

    pub async fn list_resources(&self) -> Result<Arc<Vec<Resource>>, Arc<McpError>> {
        // Capability gate — an upstream that didn't advertise
        // `resources` has no `resources/list`; calling it anyway 404s
        // on most servers (e.g. the tools-only plugin fixtures).
        if !self.has_resources_cap {
            return Ok(Arc::new(Vec::new()));
        }
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::ResourcesList {
                    mcp_kind: self.mcp_kind.clone(),
                    params: ListResourcesRequest { cursor: None },
                },
                headers,
            )
            .await
            .map_err(Arc::new)?;
        match response.payload {
            server_response::Payload::ResourcesList { result, .. } => {
                Ok(Arc::new(unwrap_rpc(&self.url, result).map_err(Arc::new)?.resources))
            }
            other => Err(Arc::new(variant_mismatch(&self.url, "resources_list", &other))),
        }
    }

    pub async fn call_tool(
        &self,
        params: &CallToolRequestParams,
    ) -> Result<CallToolResult, McpError> {
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::ToolsCall {
                    mcp_kind: self.mcp_kind.clone(),
                    params: params.clone(),
                },
                headers,
            )
            .await?;
        match response.payload {
            server_response::Payload::ToolsCall { result, .. } => unwrap_rpc(&self.url, result),
            other => Err(variant_mismatch(&self.url, "tools_call", &other)),
        }
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::ResourcesRead {
                    mcp_kind: self.mcp_kind.clone(),
                    params: ReadResourceRequestParams {
                        uri: uri.to_string(),
                    },
                },
                headers,
            )
            .await?;
        match response.payload {
            server_response::Payload::ResourcesRead { result, .. } => unwrap_rpc(&self.url, result),
            other => Err(variant_mismatch(&self.url, "resources_read", &other)),
        }
    }

    pub async fn delete(&self) -> Result<(), McpError> {
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::SessionTerminate {
                    mcp_kind: self.mcp_kind.clone(),
                },
                headers,
            )
            .await?;
        match response.payload {
            server_response::Payload::SessionTerminate { result, .. } => unwrap_rpc(&self.url, result),
            other => Err(variant_mismatch(&self.url, "session_terminate", &other)),
        }
    }

    pub fn set_on_tools_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.channel
            .set_tools_list_changed(self.mcp_kind.clone(), Arc::new(callback));
    }

    pub fn set_on_resources_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.channel
            .set_resources_list_changed(self.mcp_kind.clone(), Arc::new(callback));
    }

    pub async fn set_extra_headers(&self, extras: IndexMap<String, String>) {
        *self.extra_headers.write().await = extras;
    }
}

/// A per-upstream handle: HTTP [`Connection`] or WS [`WsUpstream`]. Exposes
/// exactly the surface [`crate::session::Session`] + `handle_delete` use.
#[derive(Debug)]
pub enum Upstream {
    Http(Connection),
    Ws(WsUpstream),
}

impl Upstream {
    pub fn url(&self) -> &str {
        match self {
            Upstream::Http(c) => &c.url,
            Upstream::Ws(w) => &w.url,
        }
    }

    pub fn session_id(&self) -> &str {
        match self {
            Upstream::Http(c) => &c.session_id,
            Upstream::Ws(w) => &w.session_id,
        }
    }

    /// Upstream `server_info.name` — used to derive the session's routing
    /// prefix. (`Connection` exposes it via `initialize_result`.)
    pub fn server_name(&self) -> &str {
        match self {
            Upstream::Http(c) => &c.initialize_result.server_info.name,
            Upstream::Ws(w) => &w.server_name,
        }
    }

    /// Upstream `server_info.version` — the prefix collision tie-breaker.
    pub fn server_version(&self) -> &str {
        match self {
            Upstream::Http(c) => &c.initialize_result.server_info.version,
            Upstream::Ws(w) => &w.server_version,
        }
    }

    pub async fn list_tools(&self) -> Result<Arc<Vec<Tool>>, Arc<McpError>> {
        match self {
            Upstream::Http(c) => c.list_tools().await,
            Upstream::Ws(w) => w.list_tools().await,
        }
    }

    pub async fn list_resources(&self) -> Result<Arc<Vec<Resource>>, Arc<McpError>> {
        match self {
            Upstream::Http(c) => c.list_resources().await,
            Upstream::Ws(w) => w.list_resources().await,
        }
    }

    pub async fn call_tool(
        &self,
        params: &CallToolRequestParams,
    ) -> Result<CallToolResult, McpError> {
        match self {
            Upstream::Http(c) => c.call_tool(params).await,
            Upstream::Ws(w) => w.call_tool(params).await,
        }
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        match self {
            Upstream::Http(c) => c.read_resource(uri).await,
            Upstream::Ws(w) => w.read_resource(uri).await,
        }
    }

    pub async fn delete(&self) -> Result<(), McpError> {
        match self {
            Upstream::Http(c) => c.delete().await,
            Upstream::Ws(w) => w.delete().await,
        }
    }

    pub fn set_on_tools_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        match self {
            Upstream::Http(c) => c.set_on_tools_list_changed(callback),
            Upstream::Ws(w) => w.set_on_tools_list_changed(callback),
        }
    }

    pub fn set_on_resources_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        match self {
            Upstream::Http(c) => c.set_on_resources_list_changed(callback),
            Upstream::Ws(w) => w.set_on_resources_list_changed(callback),
        }
    }

    pub async fn set_extra_headers(&self, extras: IndexMap<String, String>) {
        match self {
            Upstream::Http(c) => c.set_extra_headers(extras).await,
            Upstream::Ws(w) => w.set_extra_headers(extras).await,
        }
    }
}

/// Parse a `ws://objectiveai` / `ws:///owner/name/version/mcp` URL into
/// its [`McpKind`]. Returns `None` for any other shape.
pub fn parse_ws_mcp_kind(url: &str) -> Option<McpKind> {
    let rest = url.strip_prefix("ws://")?;
    // Drop any `?query` (plugin args ride there, parsed separately).
    let rest = rest.split('?').next().unwrap_or(rest);
    // `ws://objectiveai` → host "objectiveai", no path.
    if rest == "objectiveai" {
        return Some(McpKind::ObjectiveAi);
    }
    // `ws:///owner/name/version/mcp` → empty host, leading '/'.
    let path = rest.strip_prefix('/')?;
    let parts: Vec<&str> = path.split('/').collect();
    if let [owner, name, version, mcp] = parts.as_slice() {
        if !owner.is_empty() && !name.is_empty() && !version.is_empty() && !mcp.is_empty() {
            return Some(McpKind::Other {
                owner: (*owner).to_string(),
                name: (*name).to_string(),
                version: (*version).to_string(),
                mcp: (*mcp).to_string(),
            });
        }
    }
    None
}

/// `initialize` a `ws://` upstream over `channel` and build its
/// [`WsUpstream`]. `headers` is the full set sent on the `initialize`
/// request — the session-global transient identity headers, plus (on
/// resume) the upstream `Mcp-Session-Id` and any auth. `args` carries
/// plugin init arguments (empty for `objectiveai`).
pub async fn connect_ws(
    channel: ReverseChannel,
    url: String,
    mcp_kind: McpKind,
    args: IndexMap<String, Option<String>>,
    mut headers: IndexMap<String, String>,
) -> Result<WsUpstream, McpError> {
    let response = channel
        .request(
            server_request::Payload::Initialize {
                mcp_kind: mcp_kind.clone(),
                params: InitializeRequest { args },
            },
            headers.clone(),
        )
        .await?;
    let reply = match response.payload {
        server_response::Payload::Initialize { result, .. } => unwrap_rpc(&url, result)?,
        other => return Err(variant_mismatch(&url, "initialize", &other)),
    };
    // The per-request stamped set drops the resume `Mcp-Session-Id`
    // ([`WsUpstream::headers`] re-adds whatever the upstream just minted)
    // but keeps the transient identity + auth so the post-init health
    // probe + every later call still pass the conduit's transient check.
    headers.shift_remove(crate::upstream::MCP_SESSION_ID_KEY);
    let has_tools_cap = reply.result.capabilities.tools.is_some();
    let has_resources_cap = reply.result.capabilities.resources.is_some();
    Ok(WsUpstream {
        channel,
        mcp_kind,
        url,
        session_id: reply.mcp_session_id,
        server_name: reply.result.server_info.name,
        server_version: reply.result.server_info.version,
        has_tools_cap,
        has_resources_cap,
        // The connect-time set (per-URL ∪ dial-time identity) is the
        // persistent base; the transient overlay starts empty and is
        // filled by the first `set_extra_headers`. Mirrors the SDK
        // `Connection`, where connect headers are the base and
        // `extra_headers` begins empty.
        base_headers: headers,
        extra_headers: RwLock::new(IndexMap::new()),
    })
}

fn unwrap_rpc<R>(url: &str, result: JsonRpcResult<R>) -> Result<R, McpError> {
    match result {
        JsonRpcResult::Ok { result } => Ok(result),
        JsonRpcResult::Err {
            code,
            message,
            data,
        } => Err(McpError::JsonRpc {
            url: url.to_string(),
            code,
            message,
            data,
        }),
    }
}

fn transport_error(message: &str) -> McpError {
    McpError::MalformedResponse {
        url: "ws".to_string(),
        message: message.to_string(),
    }
}

fn variant_mismatch(url: &str, expected: &str, got: &server_response::Payload) -> McpError {
    McpError::MalformedResponse {
        url: url.to_string(),
        message: format!(
            "reverse channel returned wrong payload variant: expected {expected}, got {}",
            got_variant_name(got),
        ),
    }
}

fn got_variant_name(p: &server_response::Payload) -> &'static str {
    use server_response::Payload as P;
    match p {
        P::Initialize { .. } => "initialize",
        P::ToolsList { .. } => "tools_list",
        P::ToolsCall { .. } => "tools_call",
        P::ResourcesList { .. } => "resources_list",
        P::ResourcesRead { .. } => "resources_read",
        P::SessionTerminate { .. } => "session_terminate",
        P::ReadMessageQueue(_) => "read_message_queue",
        P::Retrieve(_) => "retrieve",
    }
}
