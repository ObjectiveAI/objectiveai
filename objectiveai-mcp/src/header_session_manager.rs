//! Custom rmcp `SessionManager`. Two non-default behaviors that
//! together make session-id-as-identity a soft notion:
//!
//! 1. **`has_session` always returns `Ok(true)`.** Tower never
//!    401s. Any session id the client presents is treated as valid
//!    for routing purposes; the manager itself decides on a
//!    per-message basis whether to lazily mint a worker.
//! 2. **Lazy `(handle, worker)` mint on first POST.** When tower
//!    routes a request through `create_stream` or `accept_message`
//!    for an id the inner `LocalSessionManager` doesn't currently
//!    hold, we pull the six `X-OBJECTIVEAI-*` headers off the
//!    current message's injected [`http::request::Parts`], register
//!    the resulting [`AgentArguments`] in the registry, spawn the
//!    worker + service end, and drive the worker past its initial
//!    `SessionEvent::InitializeRequest` wait state with a synthetic
//!    stub. The original message then delegates to the inner
//!    manager and rides through as if the session had existed all
//!    along.
//!
//! Net effect: the CLI keeps re-sending the six headers on every
//! connect / reconnect; the server keeps state in memory only; a
//! process restart silently rebuilds the bag from the next
//! request's headers; reconnect with a NEW header set FULL-REPLACES
//! the prior bag (missing keys become `None` on the new
//! `AgentArguments`).
//!
//! Direct adaptation of
//! `psychological-operations-x-api-mcp::HeaderSessionManager`,
//! with the header set swapped for our six `X-OBJECTIVEAI-*` keys
//! and `SessionState` replaced by [`AgentArguments`].

use std::sync::Arc;

use futures::Stream;
use objectiveai_sdk::agent::ClientObjectiveaiMcpEntry;
use objectiveai_sdk::cli::command::{AgentArguments, CommandExecutor, plugins, tools};
use rmcp::model::{
    ClientCapabilities, ClientJsonRpcMessage, ClientRequest, GetExtensions, Implementation,
    InitializeRequestParams, JsonRpcRequest, JsonRpcVersion2_0, NumberOrString, ProtocolVersion,
    Request, ServerJsonRpcMessage,
};
use rmcp::service::serve_server;
use rmcp::transport::TransportAdapterIdentity;
use rmcp::transport::WorkerTransport;
use rmcp::transport::streamable_http_server::session::SessionManager;
use rmcp::transport::streamable_http_server::session::local::{
    LocalSessionManager, LocalSessionManagerError, SessionConfig, SessionError,
    create_local_session,
};
use rmcp::transport::streamable_http_server::session::{ServerSseMessage, SessionId};

use crate::agent_args_registry::{AgentArgumentsRegistry, SessionState};
use crate::objectiveai::ObjectiveAiMcpCli;

/// Lowercase HTTP header names the conduit stamps on every outbound
/// MCP request (initialize + tool calls). Order matches the field
/// layout in [`AgentArguments`]. Setter pulls each present value
/// onto the matching slot; missing → field stays `None` (which is
/// the FULL-REPLACE behavior on the registry's `Arc::new(args)`).
const HEADER_TO_FIELD: [(&str, fn(&mut AgentArguments, String)); 6] = [
    (
        "x-objectiveai-agent-instance-hierarchy",
        |a, v| a.agent_instance_hierarchy = Some(v),
    ),
    ("x-objectiveai-agent-id", |a, v| a.agent_id = Some(v)),
    ("x-objectiveai-agent-full-id", |a, v| a.agent_full_id = Some(v)),
    ("x-objectiveai-agent-remote", |a, v| a.agent_remote = Some(v)),
    ("x-objectiveai-response-id", |a, v| a.response_id = Some(v)),
    ("x-objectiveai-response-ids", |a, v| a.response_ids = Some(v)),
];

#[derive(Debug, Clone)]
pub struct HeaderSessionManager<E> {
    inner: Arc<LocalSessionManager>,
    registry: Arc<AgentArgumentsRegistry>,
    /// Used by [`Self::ensure_session`] to spawn a service end onto
    /// each lazily-created worker.
    service: ObjectiveAiMcpCli<E>,
    /// Startup-captured tool manifest list, used to validate the
    /// optional `X-OBJECTIVEAI-MCP-TOOLS` set at connect time.
    tools_list: Arc<Vec<tools::list::ResponseItem>>,
    /// Startup-captured plugin manifest list, used to validate the
    /// optional `X-OBJECTIVEAI-MCP-PLUGINS` set at connect time.
    plugins_list: Arc<Vec<plugins::list::ResponseItem>>,
}

impl<E> HeaderSessionManager<E>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    pub fn new(
        registry: Arc<AgentArgumentsRegistry>,
        service: ObjectiveAiMcpCli<E>,
        tools_list: Arc<Vec<tools::list::ResponseItem>>,
        plugins_list: Arc<Vec<plugins::list::ResponseItem>>,
    ) -> Self {
        Self {
            inner: Arc::new(LocalSessionManager::default()),
            registry,
            service,
            tools_list,
            plugins_list,
        }
    }

    /// Make sure the inner `LocalSessionManager` has a handle for
    /// `id`. If it already does, no-op. Otherwise extract the six
    /// `X-OBJECTIVEAI-*` headers from the current message, record
    /// the resulting [`AgentArguments`], mint a worker, attach a
    /// service, and feed a synthetic initialize so the worker is
    /// ready to receive the real client message in its main loop.
    async fn ensure_session(
        &self,
        id: &SessionId,
        message: &ClientJsonRpcMessage,
    ) -> Result<(), LocalSessionManagerError> {
        if self.inner.has_session(id).await? {
            return Ok(());
        }

        let mut args = extract_agent_args(message);
        // Stamp the rmcp session id onto the bag so downstream tool /
        // plugin subprocesses see this connection's `Mcp-Session-Id`
        // as their `MCP_SESSION_ID` env. Identifies the calling agent
        // at the tool boundary (e.g. `count-tool` keys its per-caller
        // counter on it).
        args.mcp_session_id = Some(id.to_string());

        let (mcp_root, mcp_tools, mcp_plugins) = extract_mcp_filter(message)?;
        validate_mcp_filter(
            mcp_tools.as_deref(),
            mcp_plugins.as_deref(),
            &self.tools_list,
            &self.plugins_list,
        )?;

        let state = Arc::new(SessionState {
            args,
            mcp_root,
            mcp_tools,
            mcp_plugins,
        });
        self.registry.record(id.clone(), state).await;

        let (handle, worker) = create_local_session(id.clone(), SessionConfig::default());
        let transport = WorkerTransport::spawn(worker);

        // Service-side task. When the service ends (worker died,
        // transport closed) drop the entry from both maps. Cleanup
        // mirrors rmcp's tower path at
        // `streamable_http_server/tower.rs:392-416`.
        let svc = self.service.clone();
        let id_for_close = id.clone();
        let registry_for_close = self.registry.clone();
        let inner_for_close = self.inner.clone();
        tokio::spawn(async move {
            let res =
                serve_server::<_, _, _, TransportAdapterIdentity>(svc, transport).await;
            if let Ok(svc) = res {
                let _ = svc.waiting().await;
            }
            let _ = registry_for_close.remove(&id_for_close).await;
            inner_for_close
                .sessions
                .write()
                .await
                .remove(&id_for_close);
        });

        // Drive the worker past its initial
        // `SessionEvent::InitializeRequest` wait state. The
        // response is discarded; the real client's subsequent
        // request (if any) drives the actual work through the
        // worker's main loop.
        handle
            .initialize(synthetic_initialize_message())
            .await
            .map_err(|e| {
                error_invalid_input(format!("synthetic initialize: {e}"))
            })?;

        self.inner.sessions.write().await.insert(id.clone(), handle);
        Ok(())
    }
}

impl<E> SessionManager for HeaderSessionManager<E>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    type Error = LocalSessionManagerError;
    type Transport = <LocalSessionManager as SessionManager>::Transport;

    async fn create_session(&self) -> Result<(SessionId, Self::Transport), Self::Error> {
        self.inner.create_session().await
    }

    async fn initialize_session(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<ServerJsonRpcMessage, Self::Error> {
        // No-session-id POST path: extract headers, FULL-REPLACE
        // registry, delegate. The inner already has the handle from
        // its own `create_session` (called by tower right before
        // this).
        let mut args = extract_agent_args(&message);
        // Stamp the freshly-minted rmcp session id onto the bag so
        // downstream tool / plugin subprocesses see this connection's
        // `Mcp-Session-Id` as their `MCP_SESSION_ID` env.
        args.mcp_session_id = Some(id.to_string());

        let (mcp_root, mcp_tools, mcp_plugins) = extract_mcp_filter(&message)?;
        validate_mcp_filter(
            mcp_tools.as_deref(),
            mcp_plugins.as_deref(),
            &self.tools_list,
            &self.plugins_list,
        )?;

        let state = Arc::new(SessionState {
            args,
            mcp_root,
            mcp_tools,
            mcp_plugins,
        });
        self.registry.record(id.clone(), state).await;
        self.inner.initialize_session(id, message).await
    }

    /// Always `Ok(true)`. Tower's reject-with-401 path never fires
    /// for us; the validity of a session id is established lazily
    /// by [`Self::ensure_session`] reading headers off the very
    /// request that uses it.
    async fn has_session(&self, _id: &SessionId) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn close_session(&self, id: &SessionId) -> Result<(), Self::Error> {
        let _ = self.registry.remove(id).await;
        self.inner.close_session(id).await
    }

    async fn create_stream(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        self.ensure_session(id, &message).await?;
        self.inner.create_stream(id, message).await
    }

    async fn accept_message(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<(), Self::Error> {
        self.ensure_session(id, &message).await?;
        self.inner.accept_message(id, message).await
    }

    async fn create_standalone_stream(
        &self,
        id: &SessionId,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        // GET path: no message, no headers we can extract here. If
        // the inner doesn't already know the session, the client
        // gets rmcp's standard "session not found" from this path.
        // The conduit uses POST exclusively, so this branch is a
        // no-op for the in-process path.
        self.inner.create_standalone_stream(id).await
    }

    async fn resume(
        &self,
        id: &SessionId,
        last_event_id: String,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        // Same GET-path constraint as `create_standalone_stream`.
        self.inner.resume(id, last_event_id).await
    }
}

/// Pull the six `X-OBJECTIVEAI-*` header values off the message's
/// injected [`http::request::Parts`] extension and build an
/// [`AgentArguments`]. Missing or empty headers leave the matching
/// field as `None`. The caller `Arc`-wraps the result and inserts
/// into the registry, which is a FULL-REPLACE — so `None`s clear
/// any prior value.
fn extract_agent_args(message: &ClientJsonRpcMessage) -> AgentArguments {
    let parts = match message {
        ClientJsonRpcMessage::Request(r) => {
            r.request.extensions().get::<http::request::Parts>()
        }
        ClientJsonRpcMessage::Notification(n) => {
            n.notification.extensions().get::<http::request::Parts>()
        }
        _ => None,
    };
    let mut args = AgentArguments::default();
    if let Some(p) = parts {
        for (name, setter) in HEADER_TO_FIELD {
            if let Some(v) = p.headers.get(name).and_then(|v| v.to_str().ok()) {
                let s = v.trim();
                if !s.is_empty() {
                    setter(&mut args, s.to_string());
                }
            }
        }
    }
    args
}

/// Minimal-but-valid `initialize` JSON-RPC request used during
/// lazy [`HeaderSessionManager::ensure_session`] rehydration.
/// Drives the freshly-spawned worker past its initial
/// `SessionEvent::InitializeRequest` wait state.
/// `ServerHandler::initialize`'s default impl is idempotent
/// (`set_peer_info` overwrites on the next call), so the real
/// client's subsequent initialize — if any — wins.
fn synthetic_initialize_message() -> ClientJsonRpcMessage {
    let request = Request {
        method: Default::default(),
        params: InitializeRequestParams {
            meta: None,
            protocol_version: ProtocolVersion::V_2025_06_18,
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "objectiveai-mcp-restore-stub".into(),
                title: None,
                version: "0".into(),
                description: None,
                icons: None,
                website_url: None,
            },
        },
        extensions: Default::default(),
    };
    ClientJsonRpcMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion2_0,
        id: NumberOrString::Number(0),
        request: ClientRequest::InitializeRequest(request),
    })
}

fn error_invalid_input(msg: String) -> LocalSessionManagerError {
    LocalSessionManagerError::SessionError(SessionError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        msg,
    )))
}

/// Pull the three optional `X-OBJECTIVEAI-MCP-*` header values off
/// the message's injected [`http::request::Parts`] as a
/// `(root, tools, plugins)` triple ready to inline onto a
/// [`SessionState`].
///
/// - `x-objectiveai-mcp-root`: `"true"` / `"false"` ⇒ matching bool.
///   Header absent ⇒ default `true`. Anything else ⇒
///   `error_invalid_input`.
/// - `x-objectiveai-mcp-tools` / `x-objectiveai-mcp-plugins`: a JSON
///   array of `{owner, name, version}` objects (matching the
///   [`ClientObjectiveaiMcpEntry`] wire shape stamped by the api
///   side). Header absent ⇒ `None`. Header present but malformed ⇒
///   `error_invalid_input`. Header present and well-formed ⇒
///   `Some(vec)` (validated against the installed manifest by
///   [`validate_mcp_filter`]).
fn extract_mcp_filter(
    message: &ClientJsonRpcMessage,
) -> Result<
    (
        bool,
        Option<Vec<ClientObjectiveaiMcpEntry>>,
        Option<Vec<ClientObjectiveaiMcpEntry>>,
    ),
    LocalSessionManagerError,
> {
    let parts = match message {
        ClientJsonRpcMessage::Request(r) => {
            r.request.extensions().get::<http::request::Parts>()
        }
        ClientJsonRpcMessage::Notification(n) => {
            n.notification.extensions().get::<http::request::Parts>()
        }
        _ => None,
    };
    let Some(p) = parts else {
        return Ok((true, None, None));
    };
    let mut root = true;
    let mut tools: Option<Vec<ClientObjectiveaiMcpEntry>> = None;
    let mut plugins: Option<Vec<ClientObjectiveaiMcpEntry>> = None;
    if let Some(v) = p.headers.get("x-objectiveai-mcp-root").and_then(|v| v.to_str().ok()) {
        let s = v.trim();
        root = match s {
            "true" => true,
            "false" => false,
            other => {
                return Err(error_invalid_input(format!(
                    "x-objectiveai-mcp-root must be \"true\" or \"false\", got {other:?}"
                )));
            }
        };
    }
    if let Some(v) = p.headers.get("x-objectiveai-mcp-tools").and_then(|v| v.to_str().ok()) {
        let s = v.trim();
        if !s.is_empty() {
            let parsed: Vec<ClientObjectiveaiMcpEntry> =
                serde_json::from_str(s).map_err(|e| {
                    error_invalid_input(format!(
                        "x-objectiveai-mcp-tools: invalid JSON ({e})"
                    ))
                })?;
            tools = Some(parsed);
        }
    }
    if let Some(v) = p.headers.get("x-objectiveai-mcp-plugins").and_then(|v| v.to_str().ok()) {
        let s = v.trim();
        if !s.is_empty() {
            let parsed: Vec<ClientObjectiveaiMcpEntry> =
                serde_json::from_str(s).map_err(|e| {
                    error_invalid_input(format!(
                        "x-objectiveai-mcp-plugins: invalid JSON ({e})"
                    ))
                })?;
            plugins = Some(parsed);
        }
    }
    Ok((root, tools, plugins))
}

/// Validate that every `(owner, name, version)` triple in the
/// caller-supplied `tools` / `plugins` filter exists in the
/// startup-captured manifest lists. Missing entries reject the
/// session via `error_invalid_input` — the caller shouldn't be
/// declaring tools / plugins it can't reach. `None` filter ⇒ no
/// check.
fn validate_mcp_filter(
    tools: Option<&[ClientObjectiveaiMcpEntry]>,
    plugins: Option<&[ClientObjectiveaiMcpEntry]>,
    tools_list: &[tools::list::ResponseItem],
    plugins_list: &[plugins::list::ResponseItem],
) -> Result<(), LocalSessionManagerError> {
    if let Some(declared) = tools {
        for entry in declared {
            let found = tools_list.iter().any(|t| {
                t.owner == entry.owner
                    && t.name == entry.name
                    && t.version == entry.version
            });
            if !found {
                return Err(error_invalid_input(format!(
                    "tool {}/{}@{} not installed",
                    entry.owner, entry.name, entry.version
                )));
            }
        }
    }
    if let Some(declared) = plugins {
        for entry in declared {
            let found = plugins_list.iter().any(|p| {
                p.owner == entry.owner
                    && p.name == entry.name
                    && p.version == entry.version
            });
            if !found {
                return Err(error_invalid_input(format!(
                    "plugin {}/{}@{} not installed",
                    entry.owner, entry.name, entry.version
                )));
            }
        }
    }
    Ok(())
}
