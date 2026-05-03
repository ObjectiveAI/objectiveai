use std::borrow::Cow;
use std::sync::Arc;

use dashmap::DashMap;
use futures::FutureExt;
use rmcp::serve_server;
use rmcp::transport::TransportAdapterIdentity;
use rmcp::transport::WorkerTransport;
use rmcp::transport::streamable_http_server::session::SessionId;
use rmcp::transport::streamable_http_server::session::local::{
    LocalSessionManager, create_local_session,
};
use rmcp::{
    Peer, RoleServer, ServerHandler,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::tool::ToolCallContext,
    model::{
        CallToolRequestParams, CallToolResult, ClientCapabilities, ClientJsonRpcMessage,
        ClientNotification, ClientRequest, Content, Implementation, InitializeRequest,
        InitializeRequestParams, InitializeResult, InitializedNotification, NumberOrString,
        ProtocolVersion, RequestId, ServerCapabilities, ServerInfo, ServerNotification, Tool,
        ToolListChangedNotification,
    },
    service::RequestContext,
    transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService},
};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::OnceCell;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::{CancellationToken, DropGuard};

use objectiveai::functions::inventions::InventionTool;

/// Per-rmcp-session state inside the shared invention server. Tools
/// and rmcp peers are now keyed directly by rmcp's `Mcp-Session-Id`,
/// not by a parallel custom tenant id.
#[derive(Clone)]
struct SessionState {
    tool_router: Arc<RwLock<ToolRouter<InventionMcp>>>,
    peers: Arc<Mutex<Vec<Peer<RoleServer>>>>,
}

impl SessionState {
    fn new(tools: Vec<InventionTool>) -> Self {
        Self {
            tool_router: Arc::new(RwLock::new(build_router(tools))),
            peers: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// Process-wide lazy spawner for the shared invention MCP server. Mirrors
/// [`crate::agent::completions::ProxySpawner`] — first caller to
/// [`Self::get`] races on the `OnceCell` and binds a single TCP port +
/// spawns one tokio task; everyone else piggybacks on the same handle.
pub struct InventionServerSpawner {
    cell: OnceCell<Arc<InventionServerHandle>>,
    /// Optional runtime handle anchoring the server task. `None` =
    /// `tokio::spawn` against the ambient runtime (production: one
    /// long-lived runtime). `Some` is for tests where the ambient
    /// runtime is per-`#[tokio::test]` and would drop the task.
    handle: Option<tokio::runtime::Handle>,
}

impl Default for InventionServerSpawner {
    fn default() -> Self {
        Self::new()
    }
}

impl InventionServerSpawner {
    pub fn new() -> Self {
        Self {
            cell: OnceCell::new(),
            handle: None,
        }
    }

    /// Same as `new`, but the server's listener task is spawned on the
    /// supplied runtime handle so it survives even after the caller's
    /// runtime drops. Required in `#[tokio::test]` harnesses where each
    /// test owns its own runtime.
    pub fn new_with_handle(handle: tokio::runtime::Handle) -> Self {
        Self {
            cell: OnceCell::new(),
            handle: Some(handle),
        }
    }

    /// Boot the shared server on first call; return the existing handle
    /// on every subsequent call.
    pub async fn get(&self) -> std::io::Result<Arc<InventionServerHandle>> {
        self.cell
            .get_or_try_init(|| async {
                InventionServerHandle::spawn(self.handle.clone()).await
            })
            .await
            .map(Arc::clone)
    }
}

/// The live, single-port-per-process invention MCP server. Each
/// in-flight invention pre-mints an rmcp session via
/// [`InventionServerHandle::register`]; the returned [`InventionSession`]
/// carries that session id, which the orchestrator stamps as
/// `Mcp-Session-Id` on the proxy → InventionServer hop. The session id
/// also keys [`Self::sessions`] so [`InventionMcp`] can look up the
/// session's tool router on every MCP request.
pub struct InventionServerHandle {
    url: String,
    sessions: Arc<DashMap<SessionId, SessionState>>,
    /// Owned alongside (and shared with) the rmcp tower's
    /// [`StreamableHttpService`], so [`Self::register`] can pre-seed
    /// session entries and [`InventionSession::Drop`] can close them.
    rmcp_session_manager: Arc<LocalSessionManager>,
    /// Runtime to anchor per-session `serve_server` tasks. `None` →
    /// `tokio::spawn` against the ambient runtime.
    runtime_handle: Option<tokio::runtime::Handle>,
    _shutdown: DropGuard,
    _server_handle: tokio::task::AbortHandle,
}

impl InventionServerHandle {
    async fn spawn(
        runtime_handle: Option<tokio::runtime::Handle>,
    ) -> std::io::Result<Arc<Self>> {
        let ct = CancellationToken::new();
        let sessions: Arc<DashMap<SessionId, SessionState>> = Arc::new(DashMap::new());
        let rmcp_session_manager = Arc::new(LocalSessionManager::default());

        let (port_rx, server_handle) = build_and_spawn_server(
            Arc::clone(&sessions),
            Arc::clone(&rmcp_session_manager),
            ct.clone(),
            runtime_handle.clone(),
        );
        let port = port_rx
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(Arc::new(Self {
            url: format!("http://127.0.0.1:{}/mcp", port),
            sessions,
            rmcp_session_manager,
            runtime_handle,
            _shutdown: ct.drop_guard(),
            _server_handle: server_handle,
        }))
    }

    /// Pre-mint an rmcp session inside the InventionServer linked to
    /// `initial_tools`. Returns an [`InventionSession`] whose id is the
    /// rmcp session id; the orchestrator forwards that id to the proxy
    /// as the per-server `Mcp-Session-Id` header so when the proxy
    /// initializes against the InventionServer, rmcp's tower finds the
    /// alive session and dispatches normally. Tools live in
    /// [`Self::sessions`] keyed by the same id.
    pub async fn register(
        self: &Arc<Self>,
        initial_tools: Vec<InventionTool>,
    ) -> InventionSession {
        let id: SessionId = rmcp::transport::common::server_side_http::session_id();

        // 1. Per-session tool/peer state.
        self.sessions
            .insert(id.clone(), SessionState::new(initial_tools));

        // 2. Pre-seed rmcp's session table so the proxy's initialize
        //    POST (which carries `Mcp-Session-Id: <our id>`) doesn't
        //    401 on `has_session`. We mirror the work rmcp's tower
        //    would otherwise do on the no-session-id branch:
        //    create_local_session + insert handle + spawn serve_server.
        let (handle, worker) = create_local_session(
            id.clone(),
            self.rmcp_session_manager.session_config.clone(),
        );
        self.rmcp_session_manager
            .sessions
            .write()
            .await
            .insert(id.clone(), handle.clone());

        // 3. Per-session serve_server task. The shared `InventionMcp`
        //    looks up its session's tools/peers in `self.sessions` via
        //    the `Mcp-Session-Id` header on the inbound request Parts.
        let mcp = InventionMcp {
            sessions: Arc::clone(&self.sessions),
        };
        let transport = WorkerTransport::spawn(worker);
        let task = async move {
            // Keep the `RunningService` alive for the lifetime of the
            // session — its `Drop` impl cancels the worker's
            // cancellation token, which would otherwise tear down the
            // session immediately after `serve_server` returns from the
            // initialize handshake. `waiting()` parks until the service
            // task itself terminates.
            if let Ok(service) =
                serve_server::<_, _, _, TransportAdapterIdentity>(mcp, transport).await
            {
                let _ = service.waiting().await;
            }
        };
        match &self.runtime_handle {
            Some(h) => {
                h.spawn(task);
            }
            None => {
                tokio::spawn(task);
            }
        }

        // 4. Drive the MCP initialize handshake ourselves. The
        //    `LocalSessionWorker` and `serve_server` both *require* a
        //    standard initialize → initialize-response → initialized
        //    notification cycle before they will route any other
        //    request. The proxy's first POST will arrive carrying a
        //    `Mcp-Session-Id` (since we pre-seeded it server-side),
        //    which rmcp's tower routes via `create_stream` instead of
        //    `initialize_session` — that path assumes the session is
        //    already past initialize. So we synthesize the handshake
        //    here.
        //
        //    A subsequent re-initialize from the proxy is still
        //    handled gracefully: `serve_inner` is in its main request
        //    loop and routes the second `initialize` to our handler,
        //    whose impl is idempotent (`peer_info().is_none()` short-
        //    circuits the second call).
        let init_req = ClientJsonRpcMessage::request(
            ClientRequest::InitializeRequest(InitializeRequest {
                method: Default::default(),
                params: InitializeRequestParams {
                    meta: None,
                    protocol_version: ProtocolVersion::V_2025_06_18,
                    capabilities: ClientCapabilities::default(),
                    client_info: Implementation {
                        name: "objectiveai-invention-preseed".into(),
                        title: None,
                        version: env!("CARGO_PKG_VERSION").into(),
                        description: None,
                        icons: None,
                        website_url: None,
                    },
                },
                extensions: Default::default(),
            }),
            RequestId::Number(0),
        );
        let _ = handle.initialize(init_req).await;

        let initialized = ClientJsonRpcMessage::notification(
            ClientNotification::InitializedNotification(InitializedNotification {
                method: Default::default(),
                extensions: Default::default(),
            }),
        );
        let _ = handle.push_message(initialized, None).await;

        InventionSession {
            id,
            handle: Arc::clone(self),
        }
    }
}

/// Per-invention session token. Owns one [`SessionState`] slot inside
/// the shared [`InventionServerHandle`]; [`Drop`] removes both that
/// slot and the matching rmcp session.
pub struct InventionSession {
    id: SessionId,
    handle: Arc<InventionServerHandle>,
}

impl InventionSession {
    /// The shared server's URL — the same string for every session.
    /// Sessions are disambiguated server-side via the `Mcp-Session-Id`
    /// header rmcp inserts into request extensions on every request.
    pub fn url(&self) -> String {
        self.handle.url.clone()
    }

    /// rmcp session id; the orchestrator forwards this through the
    /// proxy as the per-server `Mcp-Session-Id` header so the
    /// InventionServer recognises the session and dispatches tool
    /// routing on it.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Replace this session's tool set and fan out a
    /// `notifications/tools/list_changed` to every rmcp peer in
    /// parallel. Per-invention there is normally exactly one peer
    /// (one rmcp session against the shared InventionServer for the
    /// whole invention, since the proxy reuses its upstream
    /// connection across all 5 step calls), so the broadcast is small
    /// and fast. Dead peers — those whose `send_notification` returns
    /// `Err` — are pruned from the alive list.
    pub async fn set_tools(&self, tools: Vec<InventionTool>) {
        let state = match self.handle.sessions.get(&self.id) {
            Some(e) => e.value().clone(),
            None => return, // session removed concurrently
        };

        *state.tool_router.write().await = build_router(tools);

        let peers: Vec<Peer<RoleServer>> = state.peers.lock().await.clone();
        let results = futures::future::join_all(peers.iter().map(|peer| {
            peer.send_notification(ServerNotification::ToolListChangedNotification(
                ToolListChangedNotification::default(),
            ))
        }))
        .await;

        let alive: Vec<Peer<RoleServer>> = peers
            .into_iter()
            .zip(results)
            .filter_map(|(peer, result)| result.ok().map(|()| peer))
            .collect();
        *state.peers.lock().await = alive;
    }
}

impl Drop for InventionSession {
    fn drop(&mut self) {
        // Remove the per-session tool/peer state.
        self.handle.sessions.remove(&self.id);
        // Tell rmcp to close the session: removes the LocalSessionHandle
        // from LocalSessionManager.sessions (drops its event_tx → the
        // session worker's event_rx returns None → worker quits → the
        // spawned `serve_server` task naturally exits).
        let mgr = Arc::clone(&self.handle.rmcp_session_manager);
        let id = self.id.clone();
        let task = async move {
            use rmcp::transport::streamable_http_server::session::SessionManager;
            let _ = mgr.close_session(&id).await;
        };
        match &self.handle.runtime_handle {
            Some(h) => {
                h.spawn(task);
            }
            None => {
                tokio::spawn(task);
            }
        }
    }
}

/// rmcp [`ServerHandler`] for the shared invention server. One clone
/// per session lives inside that session's `serve_server` task; every
/// clone shares the same `sessions` map and looks up its own session's
/// state via `Mcp-Session-Id` on the inbound request Parts.
#[derive(Clone)]
struct InventionMcp {
    sessions: Arc<DashMap<SessionId, SessionState>>,
}

impl InventionMcp {
    /// Look up the per-session state for this request from the
    /// `Mcp-Session-Id` header (injected into request extensions by
    /// rmcp's `streamable_http_server::tower::handle_post`). Returns
    /// `None` if the header is missing, malformed, or names an
    /// unknown session — the handler will produce an empty / no-op
    /// response in that case rather than 500.
    fn session_state_for(&self, context: &RequestContext<RoleServer>) -> Option<SessionState> {
        let parts = context.extensions.get::<axum::http::request::Parts>()?;
        let id_str = parts.headers.get("mcp-session-id")?.to_str().ok()?;
        let id: SessionId = id_str.into();
        self.sessions.get(&id).map(|e| e.value().clone())
    }
}

/// Build a fresh [`ToolRouter`] from a list of [`InventionTool`]s. Used by
/// both initial session construction and per-step tool-set swaps.
#[inline(never)]
fn build_router(tools: Vec<InventionTool>) -> ToolRouter<InventionMcp> {
    let mut tool_router = ToolRouter::<InventionMcp>::new();

    for t in tools {
        let input_schema: serde_json::Map<String, Value> = t.parameters.into_iter().collect();

        let tool_def = Tool {
            name: Cow::Owned(t.name.to_string()),
            title: None,
            description: Some(Cow::Owned(t.description.to_string())),
            input_schema: Arc::new(input_schema),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        };

        let call_fn = t.call.clone();
        tool_router.add_route(ToolRoute::new_dyn(
            tool_def,
            move |ctx: ToolCallContext<'_, InventionMcp>| {
                let call_fn = call_fn.clone();
                let arguments = ctx
                    .arguments
                    .clone()
                    .map(Value::Object)
                    .unwrap_or(Value::Object(Default::default()));
                async move {
                    let result = call_fn(arguments).await;
                    match result {
                        Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                        Err(text) => Ok(CallToolResult::error(vec![Content::text(text)])),
                    }
                }
                .boxed()
            },
        ));
    }

    tool_router
}

impl ServerHandler for InventionMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_06_18,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
            server_info: Implementation {
                name: "objectiveai-function-invention".into(),
                title: None,
                version: env!("CARGO_PKG_VERSION").into(),
                description: None,
                icons: None,
                website_url: None,
            },
            instructions: None,
        }
    }

    fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<InitializeResult, rmcp::ErrorData>> + Send + '_
    {
        async move {
            // Mirror the default impl's peer_info handling.
            if context.peer.peer_info().is_none() {
                context.peer.set_peer_info(request);
            }
            // Capture the peer for this session's later
            // `tools/list_changed` notifications. If no session matches
            // the header, the peer is dropped — we still return a
            // healthy InitializeResult so rmcp's session lifecycle
            // isn't disrupted, but tool routing for this session will
            // produce empty / not-found responses.
            if let Some(state) = self.session_state_for(&context) {
                state.peers.lock().await.push(context.peer.clone());
            }
            Ok(self.get_info())
        }
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::ErrorData> {
        let tools = match self.session_state_for(&context) {
            Some(state) => state.tool_router.read().await.list_all(),
            None => Vec::new(),
        };
        Ok(rmcp::model::ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let state = match self.session_state_for(&context) {
            Some(s) => s,
            None => {
                return Err(rmcp::ErrorData::invalid_params(
                    "no Mcp-Session-Id matches a registered invention session",
                    None,
                ));
            }
        };
        // Clone the router under the read lock so the lock guard never
        // spans the `.call(...).await` below — guards are not Send.
        let router = state.tool_router.read().await.clone();
        let tcc = ToolCallContext::new(self, request, context);
        router.call(tcc).await
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        // get_tool is sync — we can only consult session data we can
        // reach without &context. None is a safe answer (rmcp's default
        // also returns None); the tool_handler validation that calls
        // this is best-effort, and `call_tool` re-routes through
        // session_state_for anyway.
        let _ = name;
        None
    }
}

/// Separate function to prevent rmcp generics from inflating the caller.
#[inline(never)]
fn build_and_spawn_server(
    sessions: Arc<DashMap<SessionId, SessionState>>,
    rmcp_session_manager: Arc<LocalSessionManager>,
    ct: CancellationToken,
    runtime_handle: Option<tokio::runtime::Handle>,
) -> (tokio::sync::oneshot::Receiver<u16>, tokio::task::AbortHandle) {
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();
    let ct_child = ct.child_token();

    let task = async move {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let _ = port_tx.send(port);

        // The service factory handles ad-hoc inbound sessions (any
        // initialize POST that arrives without a pre-registered
        // `Mcp-Session-Id`). Those still get routed to a fresh
        // `InventionMcp` clone, but with no session state pre-seeded
        // their `session_state_for` lookups return None → empty tool
        // lists. Real invention traffic only ever uses pre-registered
        // session ids.
        let factory_sessions = Arc::clone(&sessions);
        let service: StreamableHttpService<InventionMcp, LocalSessionManager> =
            StreamableHttpService::new(
                move || {
                    Ok(InventionMcp {
                        sessions: Arc::clone(&factory_sessions),
                    })
                },
                Arc::clone(&rmcp_session_manager),
                StreamableHttpServerConfig {
                    stateful_mode: true,
                    cancellation_token: ct_child,
                    ..Default::default()
                },
            );

        let router = axum::Router::new().fallback_service(service);
        axum::serve(listener, router).await.ok();
    };

    let handle = match runtime_handle {
        Some(h) => h.spawn(task).abort_handle(),
        None => tokio::spawn(task).abort_handle(),
    };

    (port_rx, handle)
}

#[cfg(test)]
#[path = "invention_server_tests.rs"]
mod tests;
