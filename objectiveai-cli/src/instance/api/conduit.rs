//! `ConduitMcpHandler` — reverse-attach MCP forwarder for the
//! client-app side of the conduit. Hosted by cli-stream; dispatches
//! every `server_request` frame the API pushes down to a real
//! upstream MCP server, caches one `mcp::Connection` per
//! remote-minted `Mcp-Session-Id`, and forwards each upstream
//! `notifications/{tools,resources}/list_changed` back up the WS as
//! a `client_request::Payload::McpListChanged` so the API's
//! `/objectiveai-mcp` GET-SSE stream can re-emit it standard-MCP-shaped.
//!
//! Dispatch is by typed [`server_request::Payload`] variant; the
//! arms cover the closed set of methods the API ever sends
//! (`initialize` / `tools/list` / `tools/call` / `resources/list` /
//! `resources/read` / `session_terminate`). Per-arm shape:
//! - **`Initialize`.** Decode the inbound aggregate `Mcp-Session-Id`,
//!   dial primary (when needed) and every selected plugin upstream
//!   concurrently, resuming each against its stored session id.
//!   Returns the freshly-aggregated outbound session id, which the
//!   API stamps onto its `Mcp-Session-Id` response header so the
//!   proxy adopts it.
//! - **`ToolsList` / `ResourcesList`.** Forward to primary, apply the
//!   API↔CLI control surface (filter + plugin aggregation), return
//!   the merged typed result.
//! - **`ToolsCall` / `ResourcesRead`.** Route by prefix-strip to the
//!   matching upstream (primary or plugin), forward through the
//!   typed call, return the typed result.
//! - **`SessionTerminate`.** Tear down every connection in the
//!   aggregate; the proxy's HTTP DELETE wakes here as a typed
//!   variant rather than a method string.
//!
//! `Notifier` is late-bound: the pump needs one, but the `Notifier`
//! is output of `send_streaming_ws(handler, ...)` and the handler is
//! input. The caller constructs the conduit, threads its clone into
//! `send_streaming_ws`, then calls [`ConduitMcpHandler::install_notifier`]
//! on the original handle once the notifier is in hand. Pump closures
//! read the slot at fire time; events that fire before install are
//! dropped (the window is bounded by a few statements at stream
//! startup — see the plan doc).

use dashmap::DashMap;
use indexmap::IndexMap;
use objectiveai_sdk::Notifier;
use objectiveai_sdk::cli::plugins::{Output as PluginOutput, TypedOutput as TypedPluginOutput};
use objectiveai_sdk::client_objectiveai_mcp::client_request::{McpListChanged, McpListChangedKind};
use objectiveai_sdk::client_objectiveai_mcp::server_response::{InitializeReply, JsonRpcResult};
use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};
use objectiveai_sdk::http::McpHandler;
use objectiveai_sdk::mcp::resource::{
    ListResourcesRequest, ListResourcesResult, ReadResourceRequestParams, ReadResourceResult,
};
use objectiveai_sdk::mcp::tool::{
    CallToolRequestParams, CallToolResult, ListToolsRequest, ListToolsResult, Tool,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::OnceCell;

/// Header on every API-originated request to the synthetic
/// `/objectiveai-mcp` URL when the agent declared
/// `client_objectiveai_mcp`. Base64url-no-pad JSON `{names, objectiveai_builtins}`.
/// Consumed by the per-method arms and stripped from the
/// upstream-forwarded headers.
const MCP_CONFIG_HEADER: &str = "X-OBJECTIVEAI-MCP-CONFIG";

struct ConduitState {
    connection: objectiveai_sdk::mcp::Connection,
    /// `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` of the request that dialed this
    /// upstream. Carried for wire-shape parity and diagnostic
    /// readability; the actual sweep key is `response_id` below.
    agent_instance_hierarchy: String,
    /// `X-OBJECTIVEAI-RESPONSE-ID` of the request that dialed this
    /// upstream — the per-agent-slot leaf the API minted at
    /// `proxy_request_headers` build time. Used by
    /// [`ConduitMcpHandler::select_response_ids`] for direct
    /// equality match against the streamed chunk's
    /// `agent_completion_ids()`.
    response_id: String,
}

/// One MCP connection the CLI has dialed during `initialize`. Wraps
/// the bare `mcp::Connection` so future commits can grow the
/// per-connection state (proxy routes, request handlers, listener
/// tasks, …) without churning the storage type.
struct PluginMcpState {
    connection: objectiveai_sdk::mcp::Connection,
    /// ws_session_ids that have selected this `(plugin, mcp_name)`
    /// in their most recent `tools/list` (via the `mcp_servers`
    /// field on the `X-OBJECTIVEAI-MCP-CONFIG` header). Read by the
    /// `set_on_{tools,resources}_list_changed` callbacks installed
    /// on `connection` to fan out `McpListChanged` frames per
    /// interested session. Mutated by the diff logic in the
    /// `tools/list` arm.
    interested_sessions: dashmap::DashSet<String>,
}

/// Per-`ws_session_id` state derived from inbound requests. Holds
/// the most recent plugin-mcp selection (for diff-based
/// `interested_sessions` maintenance) and the primary upstream's
/// `mcp_session_id` (recorded the first time `initialize` lands for
/// this ws_session_id, used as the `mcp_session_id` on
/// `McpListChanged` frames fanned out from plugin upstreams).
struct SessionState {
    last_selected: std::sync::Mutex<Vec<PluginUpstreamKey>>,
    /// Aggregate `Mcp-Session-Id` minted on this ws_session_id's
    /// first `initialize` — base62 of [`AggregateSession`]. Used by
    /// `fan_list_changed` as the outbound `mcp_session_id` field on
    /// every `McpListChanged` frame so the API's GET-SSE handler
    /// routes events to the proxy's subscriber correctly.
    mcp_session_id: OnceLock<String>,
}

#[derive(Clone)]
pub struct ConduitMcpHandler {
    inner: Arc<Inner>,
}

struct Inner {
    /// Configured remote MCP URL (e.g. `https://mcp.example.com`).
    /// `None` ⇒ MCP isn't configured for this invocation; every
    /// request 501s the same way `objectiveai_sdk::http::RejectHandler`
    /// would.
    mcp_url: Option<String>,
    client: objectiveai_sdk::mcp::Client,
    connections: DashMap<String, Arc<ConduitState>>,
    /// Late-bound: filled by [`ConduitMcpHandler::install_notifier`]
    /// after the WS-creating call returns the notifier. Pump
    /// closures read it at fire time.
    notifier: OnceLock<Notifier>,
    /// Filesystem root for resolving installed plugin/tool manifest
    /// names — used by the `tools/list` filter to recognize
    /// `objectiveai-mcp` built-ins (any returned tool not in this set
    /// is presumed a built-in when the allow-list's
    /// `objectiveai_builtins` flag is set). `None` means filesystem
    /// is unavailable; the `objectiveai_builtins` flag effectively
    /// becomes a no-op (only explicit names match).
    config_base_dir: Option<PathBuf>,
    /// Lazy cache of installed plugin + tool manifest names. Populated
    /// on first `tools/list` arrival with the `objectiveai_builtins`
    /// flag set. Empty `HashSet` when filesystem is unavailable or
    /// nothing is installed.
    installed_names: OnceCell<HashSet<String>>,
    /// MCP connections the CLI has dialed during `initialize`,
    /// keyed by `(plugin_name, mcp_name)` — the same vocabulary the
    /// API uses on the wire. Populated by [`dial_plugin_upstream`]
    /// when the inbound `initialize` aggregate enumerates a plugin
    /// upstream. Lives for the lifetime of `Inner`; entries drop with
    /// the WS session, which tears down each `Connection`'s SSE
    /// listener and HTTP stream.
    ///
    /// Consumed by `tools/list` aggregation (when the per-session
    /// selection lists this `(plugin, mcp_name)`) and by the
    /// per-connection `set_on_{tools,resources}_list_changed`
    /// callbacks installed at dial time, which fan list_changed
    /// events out to every `ws_session_id` in `interested_sessions`.
    plugin_mcp_connections: DashMap<PluginUpstreamKey, Arc<PluginMcpState>>,
    /// `response_id → sibling group` map. Populated at every dial
    /// site by inserting one entry per id in the dial's
    /// `X-OBJECTIVEAI-RESPONSE-IDS` header, all pointing at the same
    /// `Arc<Vec<String>>`. Consumed by
    /// [`ConduitMcpHandler::select_response_ids`]: when a streamed
    /// chunk yields a response_id, its sibling group is looked up
    /// and the losers are evicted. Entries are removed as their
    /// groups are processed so a re-fire on the same winner is a
    /// no-op.
    response_id_groups: DashMap<String, Arc<Vec<String>>>,
    /// Per-`ws_session_id` `SessionState`, lazily created on first
    /// inbound request that carries an `X-OBJECTIVEAI-RESPONSE-ID`
    /// (or first `initialize` we see). Tracks the most recent
    /// plugin-mcp selection and the primary upstream's
    /// `mcp_session_id` for `list_changed` routing.
    sessions: DashMap<String, Arc<SessionState>>,
}

impl ConduitMcpHandler {
    /// Construct a handler that dials the given URL on first use.
    /// `mcp_url = None` makes every `handle()` call reject with 501.
    /// `config_base_dir` is the filesystem root the CLI consults to
    /// recognize objectiveai-mcp built-ins for the `tools/list`
    /// filter — `None` keeps the filter pure-explicit-names.
    pub fn new(mcp_url: Option<String>, config_base_dir: Option<PathBuf>) -> Self {
        let http = reqwest::Client::builder()
            .build()
            .expect("reqwest::Client::build is infallible without rustls toggles");
        let client = objectiveai_sdk::mcp::Client::new(
            http,
            "objectiveai-cli-stream-conduit".to_string(),
            String::new(),
            String::new(),
            Duration::from_secs(30),
            Duration::from_secs(1),
            Duration::from_secs(1),
            0.5,
            2.0,
            Duration::from_secs(30),
            Duration::from_secs(30),
            Duration::from_secs(60),
        );
        Self {
            inner: Arc::new(Inner {
                mcp_url,
                client,
                connections: DashMap::new(),
                notifier: OnceLock::new(),
                config_base_dir,
                installed_names: OnceCell::new(),
                plugin_mcp_connections: DashMap::new(),
                response_id_groups: DashMap::new(),
                sessions: DashMap::new(),
            }),
        }
    }

    /// Install the `Notifier` the list-changed pump uses to push
    /// `McpListChanged` frames up the WS. Idempotent — first set
    /// wins; later calls are no-ops. Call once, after
    /// `send_streaming_ws` returns the notifier and before the proxy
    /// could plausibly have triggered upstream `list_changed` fires.
    pub fn install_notifier(&self, notifier: Notifier) {
        let _ = self.inner.notifier.set(notifier);
    }

    async fn dial(
        &self,
        url: String,
        session_id: Option<String>,
        request_headers: &IndexMap<String, String>,
    ) -> Result<Arc<ConduitState>, objectiveai_sdk::mcp::Error> {
        let connect_headers = sanitize_connect_headers(request_headers);
        let agent_instance_hierarchy = agent_instance_hierarchy_from_headers(request_headers);
        let response_id = response_id_from_headers(request_headers);
        register_response_id_group(&self.inner, request_headers);
        let connection = self
            .inner
            .client
            .connect(url, session_id, Some(connect_headers))
            .await?;
        install_list_changed_pump(
            &connection,
            self.inner.clone(),
            connection.session_id.clone(),
        );
        Ok(Arc::new(ConduitState {
            connection,
            agent_instance_hierarchy,
            response_id,
        }))
    }


    /// For each `winner` response_id, look up its sibling group in
    /// [`Inner::response_id_groups`]. Drop every `ConduitState` and
    /// every `PluginMcpState` whose `response_id` is in
    /// `siblings − {winner}`. Subsequent calls with the same winner
    /// are no-ops because we also forget the losers' entries from
    /// the group map.
    ///
    /// Called by cli-stream's chunk consumer on every chunk with
    /// at least one id in `chunk.agent_completion_ids()`. The
    /// API's `X-OBJECTIVEAI-RESPONSE-IDS` header at dial time is
    /// what populated the group map; un-stamped requests skip
    /// sweep cleanly (group miss = no-op).
    pub fn select_response_ids(&self, winners: &std::collections::HashSet<String>) {
        for winner in winners {
            let Some(group_arc) = self.inner.response_id_groups.get(winner) else {
                continue;
            };
            let losers: std::collections::HashSet<String> = group_arc
                .value()
                .iter()
                .filter(|id| id.as_str() != winner.as_str())
                .cloned()
                .collect();
            // Release the DashMap read guard before mutating the
            // same map below.
            drop(group_arc);
            if losers.is_empty() {
                continue;
            }
            self.inner
                .connections
                .retain(|_, state| !losers.contains(&state.response_id));
            self.inner
                .plugin_mcp_connections
                .retain(|key, _| !losers.contains(&key.3));
            // Forget the losers' group entries so a chunk for one
            // of them (if any straggles in) is a clean no-op
            // instead of re-firing the sweep.
            for loser in &losers {
                self.inner.response_id_groups.remove(loser);
            }
        }
    }
}

/// Dial a plugin's MCP upstream: verify the plugin manifest declares
/// the requested `mcp_name`, spawn `<plugin> mcp <mcp_name> begin`,
/// capture the first `Mcp { url }` notification from its stdout, dial
/// that URL (resuming with `stored_session_id` when present), install
/// list-changed callbacks, and store the resulting connection under
/// `(plugin_name, mcp_name)` in `inner.plugin_mcp_connections`.
///
/// Returns the dialed connection's `session_id` on success. Any step
/// failing (manifest miss, spawn failure, timeout, plugin error,
/// dial failure) returns [`ConduitError::PluginDialFailed`] with the
/// identifying pair so the caller's error envelope is actionable.
async fn dial_plugin_upstream(
    inner: &Arc<Inner>,
    plugin_name: String,
    mcp_name: String,
    agent_instance_hierarchy: String,
    agent_id: String,
    response_id: String,
    arguments: Option<IndexMap<String, Option<String>>>,
    stored_session_id: Option<String>,
) -> Result<String, ConduitError> {
    let fail = |reason: String| ConduitError::PluginDialFailed {
        plugin_name: plugin_name.clone(),
        mcp_name: mcp_name.clone(),
        reason,
    };

    let Some(base_dir) = inner.config_base_dir.clone() else {
        return Err(fail("filesystem unavailable (no config_base_dir)".into()));
    };
    let fs =
        crate::filesystem::Client::new(Some(base_dir), None::<String>, None::<String>);

    let Some(plugin) = fs.get_plugin(&plugin_name).await else {
        return Err(fail(format!("plugin {plugin_name:?} not installed")));
    };
    if !plugin
        .manifest
        .mcp_servers
        .iter()
        .any(|s| s.name == mcp_name)
    {
        return Err(fail(format!(
            "plugin {plugin_name:?} manifest does not declare mcp server {mcp_name:?}"
        )));
    }
    let Some(exe) = fs.resolve_plugin(&plugin_name).await else {
        return Err(fail(format!("plugin {plugin_name:?} binary not found")));
    };

    let mut cmd = tokio::process::Command::new(&exe);
    cmd.arg("mcp").arg(&mcp_name).arg("begin");
    if let Some(args) = arguments.as_ref() {
        for (k, v) in args {
            cmd.arg(format!("--{k}"));
            if let Some(value) = v {
                cmd.arg(value);
            }
        }
    }
    // Agent identity for this MCP run comes from the upstream request
    // that asked for it, NOT from cli-stream's own config. The other
    // config-shaped env vars travel through the default parent-env
    // inheritance (cli already stamped them on cli-stream).
    cmd.env("OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY", &agent_instance_hierarchy);
    if !agent_id.is_empty() {
        cmd.env("OBJECTIVEAI_AGENT_ID", &agent_id);
    }
    let mut child = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| fail(format!("spawn failed: {e}")))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    use tokio::io::AsyncBufReadExt;
    let reader = tokio::io::BufReader::new(stdout);
    let mut lines = reader.lines();

    let timeout = std::time::Duration::from_secs(30);
    let begin_result = tokio::time::timeout(timeout, async {
        loop {
            let line = match lines.next_line().await {
                Ok(Some(l)) => l,
                Ok(None) => {
                    return Err::<objectiveai_sdk::cli::plugins::Mcp, String>(
                        "plugin exited without emitting mcp{url}".into(),
                    );
                }
                Err(e) => return Err(format!("plugin stdout read error: {e}")),
            };
            let out = match serde_json::from_str::<PluginOutput>(&line) {
                Ok(o) => o,
                Err(_) => continue,
            };
            match out {
                PluginOutput::Typed(TypedPluginOutput::Mcp(mcp)) => return Ok(mcp),
                PluginOutput::Typed(TypedPluginOutput::Error(err)) => {
                    return Err(format!("plugin emitted error: {}", err.message));
                }
                PluginOutput::Notification(_)
                | PluginOutput::Typed(TypedPluginOutput::Command { .. }) => {}
            }
        }
    })
    .await;

    // Hand off child + remaining IO to a detached task so the plugin
    // keeps running after we've captured the URL. Drains stdout and
    // forwards stderr so pipe buffers don't fill up.
    tokio::spawn(async move {
        let stderr_task = tokio::spawn(forward_stderr(stderr));
        while let Ok(Some(_)) = lines.next_line().await {}
        let _ = stderr_task.await;
        let _ = child.wait().await;
    });

    let mcp = match begin_result {
        Ok(Ok(mcp)) => mcp,
        Ok(Err(message)) => return Err(fail(message)),
        Err(_) => return Err(fail("plugin mcp begin timed out".into())),
    };

    let connection = inner
        .client
        .connect(mcp.url, stored_session_id, None)
        .await
        .map_err(|e| fail(format!("connect: {e}")))?;
    let session_id = connection.session_id.clone();

    let state = Arc::new(PluginMcpState {
        connection,
        interested_sessions: dashmap::DashSet::new(),
    });
    let inner_t = inner.clone();
    let state_t = state.clone();
    state.connection.set_on_tools_list_changed(move || {
        fan_list_changed(&inner_t, &state_t, McpListChangedKind::Tools);
    });
    let inner_r = inner.clone();
    let state_r = state.clone();
    state.connection.set_on_resources_list_changed(move || {
        fan_list_changed(&inner_r, &state_r, McpListChangedKind::Resources);
    });
    let args_canonical = arguments
        .as_ref()
        .map(|m| serde_json::to_string(m).unwrap_or_default())
        .unwrap_or_default();
    // Cache key uses both `agent_instance_hierarchy` (slot 2, diagnostic) and
    // `response_id` (slot 3, uniqueness). The streamed chunk's
    // `agent_completion_ids()` are response_ids, so the group-
    // aware sweep in `select_response_ids` matches slot 3 directly.
    inner.plugin_mcp_connections.insert(
        (
            plugin_name,
            mcp_name,
            agent_instance_hierarchy,
            response_id,
            args_canonical,
        ),
        state,
    );

    Ok(session_id)
}

async fn forward_stderr(mut stderr: tokio::process::ChildStderr) {
    use tokio::io::AsyncReadExt;
    let mut buf = [0u8; 4096];
    loop {
        match stderr.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                use std::io::Write;
                let _ = std::io::stderr().write_all(&buf[..n]);
            }
        }
    }
}

impl McpHandler for ConduitMcpHandler {
    async fn handle(&self, request: server_request::Request) -> server_response::Response {
        let id = request.id.clone();
        let config = read_mcp_config_header(&request.headers);

        // `Initialize` is the only path that dials upstreams. All
        // routing inside `dispatch_initialize` is driven by the
        // inbound aggregate `Mcp-Session-Id` (decoded there) plus
        // the `X-OBJECTIVEAI-MCP-CONFIG` selection.
        if matches!(request.payload, server_request::Payload::Initialize) {
            let payload = dispatch_initialize(&self.inner, config.as_ref(), &request).await;
            return server_response::Response { id, payload };
        }

        // `SessionTerminate` does not need a primary lookup — the
        // teardown logic walks the aggregate itself.
        if matches!(request.payload, server_request::Payload::SessionTerminate) {
            let payload = dispatch_session_terminate(&self.inner, &request).await;
            return server_response::Response { id, payload };
        }

        // Every other variant needs primary (if the aggregate names
        // one): decode the inbound `Mcp-Session-Id`, look up
        // primary's `ConduitState` (or re-dial it on a CLI restart),
        // then dispatch the typed variant.
        let aggregate_id = request
            .headers
            .iter()
            .find_map(|(k, v)| k.eq_ignore_ascii_case("mcp-session-id").then(|| v.clone()));
        let Some(aggregate_id) = aggregate_id else {
            return server_response::Response {
                id,
                payload: error_payload_for(
                    &request.payload,
                    -32600,
                    "missing Mcp-Session-Id".to_string(),
                ),
            };
        };
        let Some(aggregate) = AggregateSession::decode(&aggregate_id) else {
            return server_response::Response {
                id,
                payload: error_payload_for(
                    &request.payload,
                    -32600,
                    "invalid Mcp-Session-Id (decode failed)".to_string(),
                ),
            };
        };
        let primary_state: Option<Arc<ConduitState>> = match aggregate.primary {
            Some(primary_sid) => {
                if let Some(existing) = self.inner.connections.get(&primary_sid) {
                    Some(existing.clone())
                } else {
                    let Some(mcp_url) = self.inner.mcp_url.as_ref() else {
                        return server_response::Response {
                            id,
                            payload: error_payload_for(
                                &request.payload,
                                -32601,
                                "this client has no MCP server configured (pass --mcp-address)"
                                    .to_string(),
                            ),
                        };
                    };
                    match self
                        .dial(mcp_url.clone(), Some(primary_sid.clone()), &request.headers)
                        .await
                    {
                        Ok(st) => {
                            self.inner.connections.insert(primary_sid, st.clone());
                            Some(st)
                        }
                        Err(e) => {
                            return server_response::Response {
                                id,
                                payload: error_payload_for(
                                    &request.payload,
                                    -32603,
                                    format!("conduit: connect (resume): {e}"),
                                ),
                            };
                        }
                    }
                }
            }
            None => None,
        };

        let payload = dispatch_typed(
            &self.inner,
            primary_state.as_deref(),
            config.as_ref(),
            &request,
        )
        .await;
        server_response::Response { id, payload }
    }
}

/// Wire `set_on_{tools,resources}_list_changed` to fire-and-forget
/// notifier sends. Closures read the late-bound `Notifier` from the
/// `Inner`'s `OnceLock` at fire time — events that fire before
/// `install_notifier` is called are dropped silently.
fn install_list_changed_pump(
    connection: &objectiveai_sdk::mcp::Connection,
    inner: Arc<Inner>,
    mcp_session_id: String,
) {
    let inner_tools = inner.clone();
    let session_tools = mcp_session_id.clone();
    connection.set_on_tools_list_changed(move || {
        let Some(notifier) = inner_tools.notifier.get().cloned() else {
            return;
        };
        let mcp_session_id = session_tools.clone();
        tokio::spawn(async move {
            let _ = notifier
                .notify_list_changed(McpListChanged {
                    mcp_session_id,
                    kind: McpListChangedKind::Tools,
                })
                .await;
        });
    });

    let inner_resources = inner;
    let session_resources = mcp_session_id;
    connection.set_on_resources_list_changed(move || {
        let Some(notifier) = inner_resources.notifier.get().cloned() else {
            return;
        };
        let mcp_session_id = session_resources.clone();
        tokio::spawn(async move {
            let _ = notifier
                .notify_list_changed(McpListChanged {
                    mcp_session_id,
                    kind: McpListChangedKind::Resources,
                })
                .await;
        });
    });
}

/// Fan a `list_changed` event from a plugin MCP connection out to
/// every interested ws_session_id via the WS notifier. The frame's
/// `mcp_session_id` is the PRIMARY upstream's session id for the
/// session (recorded during `initialize` handling) — that's the id
/// the API's GET-SSE handler uses to route the event to the proxy's
/// subscriber.
///
/// Drops events for sessions that haven't yet completed `initialize`
/// (no primary mcp_session_id recorded); the next `tools/list` for
/// that session will refresh state anyway. Drops the whole fan-out
/// if the WS notifier isn't installed yet.
fn fan_list_changed(inner: &Arc<Inner>, state: &Arc<PluginMcpState>, kind: McpListChangedKind) {
    let Some(notifier) = inner.notifier.get().cloned() else {
        return;
    };
    let interested: Vec<String> = state
        .interested_sessions
        .iter()
        .map(|s| s.clone())
        .collect();
    for ws_session_id in interested {
        let Some(sess) = inner.sessions.get(&ws_session_id) else {
            continue;
        };
        let Some(mcp_session_id) = sess.mcp_session_id.get().cloned() else {
            continue;
        };
        let notifier = notifier.clone();
        tokio::spawn(async move {
            let _ = notifier
                .notify_list_changed(McpListChanged {
                    mcp_session_id,
                    kind,
                })
                .await;
        });
    }
}

/// Hop-by-hop and layer-internal headers don't propagate to MCP.
fn sanitize_connect_headers(headers: &IndexMap<String, String>) -> IndexMap<String, String> {
    let mut out = headers.clone();
    for k in [
        "Host",
        "host",
        "Content-Length",
        "content-length",
        "Mcp-Session-Id",
        "mcp-session-id",
    ] {
        out.shift_remove(k);
    }
    out
}

// ───────────────────────────────────────────────────────────────────
// Per-variant dispatchers — each returns the typed
// `server_response::Payload` variant matching the inbound request.
// ───────────────────────────────────────────────────────────────────

/// `Initialize`: dial primary (when needed) and every selected plugin
/// upstream concurrently, resuming each against the inbound aggregate
/// `Mcp-Session-Id`. Encode the dialed session ids into a new
/// aggregate and return it as the `Initialize` reply; the API stamps
/// it onto its outbound `Mcp-Session-Id` response header.
///
/// First dial failure aborts the whole initialize via
/// `try_join_all`'s cancel-on-error. Partial-success connections
/// already stored in `plugin_mcp_connections` leak until the next
/// initialize overwrites — cleanup is a follow-up.
async fn dispatch_initialize(
    inner: &Arc<Inner>,
    config: Option<&McpConfig>,
    request: &server_request::Request,
) -> server_response::Payload {
    // Decode inbound aggregate. `None` on a fresh session;
    // `Some(_)` on a continuation. Each upstream's stored session
    // id rides through unchanged so the dial below can resume it.
    let inbound_aggregate: Option<AggregateSession> = request
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("mcp-session-id"))
        .and_then(|(_, v)| AggregateSession::decode(v));

    let needs_primary = config
        .map(|c| !c.names.is_empty() || c.objectiveai_builtins)
        // Header is API-stamped on every request; absence is a
        // bug. Default to dialing primary so we degrade toward
        // today's behavior rather than dropping the call.
        .unwrap_or(true);
    let plugin_entries: Vec<McpServerConfigEntry> =
        config.map(|c| c.mcp_servers.clone()).unwrap_or_default();
    let agent_instance_hierarchy = agent_instance_hierarchy_from_headers(&request.headers);
    let agent_id = agent_id_from_headers(&request.headers);
    let response_id = response_id_from_headers(&request.headers);
    // Register the sibling group exactly once for this initialize.
    // The per-agent dials below all reuse the same shared
    // `Arc<Vec<String>>` via re-insert of the same ids, which is
    // idempotent.
    register_response_id_group(inner, &request.headers);

    if !needs_primary && plugin_entries.is_empty() {
        return server_response::Payload::Initialize(JsonRpcResult::Err {
            code: -32602,
            message: "agent declaration references no upstreams".into(),
            data: None,
        });
    }

    // Primary future. Skipped when `needs_primary = false`; the
    // `expect` is unreachable in that branch.
    let stored_primary_sid = inbound_aggregate.as_ref().and_then(|a| a.primary.clone());
    let primary_headers = sanitize_connect_headers(&request.headers);
    let primary_fut = async {
        if !needs_primary {
            return Ok::<Option<String>, ConduitError>(None);
        }
        let mcp_url = inner
            .mcp_url
            .as_ref()
            .expect("primary only dialed when configured")
            .clone();
        let connection = inner
            .client
            .connect(mcp_url, stored_primary_sid, Some(primary_headers))
            .await
            .map_err(|_| ConduitError::PrimaryDialFailed)?;
        install_list_changed_pump(&connection, inner.clone(), connection.session_id.clone());
        let session_id = connection.session_id.clone();
        let agent_instance_hierarchy_for_state = agent_instance_hierarchy.clone();
        let response_id_for_state = response_id.clone();
        inner.connections.insert(
            session_id.clone(),
            Arc::new(ConduitState {
                connection,
                agent_instance_hierarchy: agent_instance_hierarchy_for_state,
                response_id: response_id_for_state,
            }),
        );
        Ok(Some(session_id))
    };

    // Plugin futures: one per `(plugin, mcp_name)` entry in the
    // agent declaration. Each resumes its upstream session if the
    // inbound aggregate carries a matching entry.
    let plugin_futs: Vec<_> = plugin_entries
        .iter()
        .cloned()
        .map(|entry| {
            let stored_sid = inbound_aggregate.as_ref().and_then(|a| {
                a.plugins
                    .iter()
                    .find(|e| e.plugin_name == entry.plugin && e.mcp_name == entry.name)
                    .map(|e| e.mcp_session_id.clone())
            });
            let inner = inner.clone();
            let agent_instance_hierarchy = agent_instance_hierarchy.clone();
            let agent_id = agent_id.clone();
            let response_id = response_id.clone();
            async move {
                dial_plugin_upstream(
                    &inner,
                    entry.plugin,
                    entry.name,
                    agent_instance_hierarchy,
                    agent_id,
                    response_id,
                    entry.arguments,
                    stored_sid,
                )
                .await
            }
        })
        .collect();

    let plugin_joined = futures::future::try_join_all(plugin_futs);
    let join_result = tokio::try_join!(primary_fut, plugin_joined);
    let (primary_sid_opt, plugin_results) = match join_result {
        Ok(v) => v,
        Err(e) => {
            return server_response::Payload::Initialize(JsonRpcResult::Err {
                code: -32603,
                message: format!("conduit: {e}"),
                data: None,
            });
        }
    };

    // Build the outbound aggregate from the dialed session ids.
    // Input ordering of `plugin_entries` is preserved through
    // `try_join_all`'s `Vec<Output>`.
    let aggregate = AggregateSession {
        primary: primary_sid_opt,
        plugins: plugin_entries
            .into_iter()
            .zip(plugin_results.into_iter())
            .map(|(entry, mcp_session_id)| AggregatePluginEntry {
                plugin_name: entry.plugin,
                mcp_name: entry.name,
                mcp_session_id,
            })
            .collect(),
    };
    let aggregate_encoded = aggregate.encode();

    // Record the aggregate on this ws_session_id's `SessionState`.
    // `list_changed` fan-out from selected plugin upstreams uses
    // this as the outbound `mcp_session_id` on every
    // `McpListChanged` frame so the API routes the event to the
    // proxy's GET-SSE subscriber correctly.
    if let Some(ws_session_id) = ws_session_id_from_headers(&request.headers) {
        let sess = get_or_create_session(inner, &ws_session_id);
        let _ = sess.mcp_session_id.set(aggregate_encoded.clone());
    }

    server_response::Payload::Initialize(JsonRpcResult::Ok {
        result: InitializeReply {
            mcp_session_id: aggregate_encoded,
        },
    })
}

/// `SessionTerminate`: drop every cached connection in the inbound
/// aggregate. The MCP-spec `DELETE` semantics of the WS-level
/// session would also delete each upstream session, but the SDK's
/// `Drop` for `Connection` tears down its SSE listener and HTTP
/// stream the moment the last `Arc` clone drops — removing the
/// state from `connections` / `plugin_mcp_connections` is enough
/// for the conduit's purposes.
async fn dispatch_session_terminate(
    inner: &Arc<Inner>,
    request: &server_request::Request,
) -> server_response::Payload {
    let aggregate_id = request
        .headers
        .iter()
        .find_map(|(k, v)| k.eq_ignore_ascii_case("mcp-session-id").then(|| v.clone()));
    if let Some(aggregate_id) = aggregate_id {
        if let Some(aggregate) = AggregateSession::decode(&aggregate_id) {
            if let Some(primary_sid) = aggregate.primary.as_ref() {
                inner.connections.remove(primary_sid);
            }
            for entry in &aggregate.plugins {
                inner.plugin_mcp_connections.retain(|key, _| {
                    !(key.0 == entry.plugin_name && key.1 == entry.mcp_name)
                });
            }
        }
    }
    server_response::Payload::SessionTerminate
}

/// Typed dispatch for `tools/list`, `tools/call`, `resources/list`,
/// `resources/read`. `Initialize` and `SessionTerminate` are routed
/// before this point. Each arm builds the corresponding
/// `server_response::Payload` variant directly.
async fn dispatch_typed(
    inner: &Arc<Inner>,
    primary: Option<&ConduitState>,
    config: Option<&McpConfig>,
    request: &server_request::Request,
) -> server_response::Payload {
    match &request.payload {
        server_request::Payload::Initialize | server_request::Payload::SessionTerminate => {
            // Handled upstream.
            unreachable!("Initialize / SessionTerminate routed elsewhere")
        }
        server_request::Payload::ToolsList(params) => {
            dispatch_tools_list(inner, primary, config, &request.headers, params.clone()).await
        }
        server_request::Payload::ToolsCall(params) => {
            dispatch_tools_call(inner, primary, config, &request.headers, params.clone()).await
        }
        server_request::Payload::ResourcesList(params) => {
            dispatch_resources_list(inner, primary, config, &request.headers, params.clone())
                .await
        }
        server_request::Payload::ResourcesRead(params) => {
            dispatch_resources_read(inner, primary, config, &request.headers, params.clone())
                .await
        }
    }
}

async fn dispatch_tools_list(
    inner: &Arc<Inner>,
    primary: Option<&ConduitState>,
    config: Option<&McpConfig>,
    headers: &IndexMap<String, String>,
    params: ListToolsRequest,
) -> server_response::Payload {
    let Some(primary) = primary else {
        return server_response::Payload::ToolsList(JsonRpcResult::Err {
            code: -32601,
            message: "method \"tools/list\" requires a primary upstream (none selected)".into(),
            data: None,
        });
    };
    let cfg = config.expect("X-OBJECTIVEAI-MCP-CONFIG header is required on tools/list");

    let primary_call = upstream_call::<ListToolsRequest, ListToolsResult>(
        &primary.connection,
        headers,
        "tools/list",
        &params,
    )
    .await;
    let mut result = match primary_call {
        Ok(JsonRpcResult::Ok { result }) => result,
        Ok(err @ JsonRpcResult::Err { .. }) => {
            return server_response::Payload::ToolsList(err);
        }
        Err(e) => {
            return server_response::Payload::ToolsList(JsonRpcResult::Err {
                code: -32603,
                message: format!("conduit: {e}"),
                data: None,
            });
        }
    };

    apply_tools_filter(inner, &mut result, cfg).await;

    let ws_session_id = ws_session_id_from_headers(headers);
    let agent_instance_hierarchy = agent_instance_hierarchy_from_headers(headers);
    let response_id = response_id_from_headers(headers);
    let selection: Vec<PluginUpstreamKey> = cfg
        .mcp_servers
        .iter()
        .map(|e| e.cache_key(&agent_instance_hierarchy, &response_id))
        .collect();
    if let Err(e) =
        aggregate_plugin_tools(inner, &mut result, &selection, ws_session_id.as_deref()).await
    {
        return server_response::Payload::ToolsList(JsonRpcResult::Err {
            code: -32603,
            message: format!("conduit: {e}"),
            data: None,
        });
    }

    server_response::Payload::ToolsList(JsonRpcResult::Ok { result })
}

async fn dispatch_resources_list(
    inner: &Arc<Inner>,
    primary: Option<&ConduitState>,
    config: Option<&McpConfig>,
    headers: &IndexMap<String, String>,
    params: ListResourcesRequest,
) -> server_response::Payload {
    let Some(primary) = primary else {
        return server_response::Payload::ResourcesList(JsonRpcResult::Err {
            code: -32601,
            message: "method \"resources/list\" requires a primary upstream (none selected)"
                .into(),
            data: None,
        });
    };
    let cfg = config.expect("X-OBJECTIVEAI-MCP-CONFIG header is required on resources/list");

    let primary_call = upstream_call::<ListResourcesRequest, ListResourcesResult>(
        &primary.connection,
        headers,
        "resources/list",
        &params,
    )
    .await;
    let mut result = match primary_call {
        Ok(JsonRpcResult::Ok { result }) => result,
        Ok(err @ JsonRpcResult::Err { .. }) => {
            return server_response::Payload::ResourcesList(err);
        }
        Err(e) => {
            return server_response::Payload::ResourcesList(JsonRpcResult::Err {
                code: -32603,
                message: format!("conduit: {e}"),
                data: None,
            });
        }
    };

    let ws_session_id = ws_session_id_from_headers(headers);
    let agent_instance_hierarchy = agent_instance_hierarchy_from_headers(headers);
    let response_id = response_id_from_headers(headers);
    let selection: Vec<PluginUpstreamKey> = cfg
        .mcp_servers
        .iter()
        .map(|e| e.cache_key(&agent_instance_hierarchy, &response_id))
        .collect();
    if let Err(e) =
        aggregate_plugin_resources(inner, &mut result, &selection, ws_session_id.as_deref()).await
    {
        return server_response::Payload::ResourcesList(JsonRpcResult::Err {
            code: -32603,
            message: format!("conduit: {e}"),
            data: None,
        });
    }

    server_response::Payload::ResourcesList(JsonRpcResult::Ok { result })
}

async fn dispatch_tools_call(
    inner: &Arc<Inner>,
    primary: Option<&ConduitState>,
    config: Option<&McpConfig>,
    headers: &IndexMap<String, String>,
    params: CallToolRequestParams,
) -> server_response::Payload {
    let cfg = config.expect("X-OBJECTIVEAI-MCP-CONFIG header is required on tools/call");

    // No primary AND no plugins → -32601 (method not available).
    if cfg.mcp_servers.is_empty() && primary.is_none() {
        return server_response::Payload::ToolsCall(JsonRpcResult::Err {
            code: -32601,
            message: "method \"tools/call\" not supported by selected upstreams (no primary)"
                .into(),
            data: None,
        });
    }

    // Routing: fan list_tools across primary + plugins, find which
    // upstream exposes the requested name, rewrite `params.name` to
    // the unprefixed form, and forward via that upstream.
    match try_route_tools_call(inner, primary, headers, cfg, params).await {
        Ok(payload) => payload,
        Err(e) => server_response::Payload::ToolsCall(JsonRpcResult::Err {
            code: -32603,
            message: format!("conduit: {e}"),
            data: None,
        }),
    }
}

async fn dispatch_resources_read(
    inner: &Arc<Inner>,
    primary: Option<&ConduitState>,
    config: Option<&McpConfig>,
    headers: &IndexMap<String, String>,
    params: ReadResourceRequestParams,
) -> server_response::Payload {
    let cfg = config.expect("X-OBJECTIVEAI-MCP-CONFIG header is required on resources/read");

    if cfg.mcp_servers.is_empty() && primary.is_none() {
        return server_response::Payload::ResourcesRead(JsonRpcResult::Err {
            code: -32601,
            message: "method \"resources/read\" not supported by selected upstreams (no primary)"
                .into(),
            data: None,
        });
    }

    match try_route_resources_read(inner, primary, headers, cfg, params).await {
        Ok(payload) => payload,
        Err(e) => server_response::Payload::ResourcesRead(JsonRpcResult::Err {
            code: -32603,
            message: format!("conduit: {e}"),
            data: None,
        }),
    }
}

/// Build a `JsonRpcResult::Err` typed into the corresponding
/// response variant for the inbound request. Used by the
/// transport-level error paths in `handle()` that need to return
/// an error before the per-variant dispatch arm runs (missing
/// `Mcp-Session-Id`, decode failure, primary dial failure).
fn error_payload_for(
    payload: &server_request::Payload,
    code: i64,
    message: String,
) -> server_response::Payload {
    let err = JsonRpcResult::<()>::Err {
        code,
        message: message.clone(),
        data: None,
    };
    // Re-build with the right phantom-type so each Payload variant
    // takes the right `JsonRpcResult<R>` shape. Since each variant
    // wraps a different R but the error variant carries no `R`
    // value, we just need to type-erase by building each per-variant.
    let _ = err;
    match payload {
        server_request::Payload::Initialize => {
            server_response::Payload::Initialize(JsonRpcResult::Err {
                code,
                message,
                data: None,
            })
        }
        server_request::Payload::ToolsList(_) => {
            server_response::Payload::ToolsList(JsonRpcResult::Err {
                code,
                message,
                data: None,
            })
        }
        server_request::Payload::ToolsCall(_) => {
            server_response::Payload::ToolsCall(JsonRpcResult::Err {
                code,
                message,
                data: None,
            })
        }
        server_request::Payload::ResourcesList(_) => {
            server_response::Payload::ResourcesList(JsonRpcResult::Err {
                code,
                message,
                data: None,
            })
        }
        server_request::Payload::ResourcesRead(_) => {
            server_response::Payload::ResourcesRead(JsonRpcResult::Err {
                code,
                message,
                data: None,
            })
        }
        server_request::Payload::SessionTerminate => {
            // SessionTerminate has no error variant in its typed
            // reply; surface as a successful ack since the failure
            // path here means "we couldn't decode the aggregate to
            // tear it down" which is effectively a no-op anyway.
            let _ = message;
            let _ = code;
            server_response::Payload::SessionTerminate
        }
    }
}

/// Raw POST through an `mcp::Connection`. Builds a JSON-RPC
/// envelope (`{jsonrpc, id, method, params}`) from the typed
/// `params`, forwards inbound headers verbatim (modulo a hop-by-hop
/// blacklist), sets `Mcp-Session-Id` to the connection's own
/// session id, parses the response body via [`parse_json_or_sse`],
/// and projects the JSON-RPC `{result|error}` shape into the
/// SDK-typed [`JsonRpcResult<R>`]. Transport-level failures (HTTP
/// errors, malformed envelopes) bubble up as [`ConduitError`].
async fn upstream_call<P, R>(
    conn: &objectiveai_sdk::mcp::Connection,
    headers: &IndexMap<String, String>,
    method: &str,
    params: &P,
) -> Result<JsonRpcResult<R>, ConduitError>
where
    P: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    // Mint a request id local to this upstream call. The proxy's
    // original JSON-RPC id rode on the inbound HTTP request to the
    // API; the API↔CLI WS link uses `request.id` for correlation,
    // and the upstream's id is independent.
    let rpc_id = uuid::Uuid::new_v4().to_string();
    let envelope = serde_json::json!({
        "jsonrpc": "2.0",
        "id": rpc_id,
        "method": method,
        "params": params,
    });

    let mut req = conn.http_client.post(&conn.url);
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("host")
            || k.eq_ignore_ascii_case("content-length")
            || k.eq_ignore_ascii_case("connection")
            || k.eq_ignore_ascii_case("accept")
            || k.eq_ignore_ascii_case("content-type")
            || k.eq_ignore_ascii_case("mcp-session-id")
            || k.eq_ignore_ascii_case(MCP_CONFIG_HEADER)
        {
            // X-OBJECTIVEAI-MCP-CONFIG is an API↔CLI signal; the
            // upstream MCP server doesn't need to see it.
            continue;
        }
        req = req.header(k, v);
    }
    req = req
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Mcp-Session-Id", &conn.session_id)
        .json(&envelope);

    let resp = req.send().await.map_err(ConduitError::Request)?;
    let resp_text = resp.text().await.map_err(ConduitError::Body)?;
    let Some(body) = parse_json_or_sse(&resp_text) else {
        return Err(ConduitError::MalformedUpstream(
            "empty or unparseable upstream response".into(),
        ));
    };

    if let Some(result) = body.get("result") {
        let typed: R = serde_json::from_value(result.clone()).map_err(|e| {
            ConduitError::MalformedUpstream(format!("decode upstream result: {e}"))
        })?;
        return Ok(JsonRpcResult::Ok { result: typed });
    }
    if let Some(err) = body.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-32603);
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("upstream returned an error envelope without a message")
            .to_string();
        let data = err.get("data").cloned();
        return Ok(JsonRpcResult::Err {
            code,
            message,
            data,
        });
    }
    Err(ConduitError::MalformedUpstream(
        "upstream response missing both `result` and `error`".into(),
    ))
}

/// Aggregate tools from the selected plugin MCP connections into the
/// primary upstream's `tools/list` response. Per the agent's
/// `client_objectiveai_mcp.plugins[i].mcp_servers` selection (carried
/// through `X-OBJECTIVEAI-MCP-CONFIG.mcp_servers`), look up each
/// matching `PluginMcpState`, call `Connection::list_tools` on it
/// concurrently, prefix each returned tool name `<mcp_name>_<tool>`
/// (mirrors `objectiveai-mcp-proxy/src/session.rs::prefix_name`'s
/// `<server>_<tool>` shape), and append to `result.tools`. If any
/// prefixed plugin tool collides with a primary tool's name, prefix
/// every primary tool with `objectiveai-mcp_` (the conventional
/// server-name for the local objectiveai-mcp upstream). Finally sort
/// the merged list by name for stable ordering.
///
/// Also reconciles `interested_sessions` on each `PluginMcpState`:
/// pairs added since the last `tools/list` for this ws_session_id
/// get this session added; removed pairs get it removed. Mutation
/// is gated by the session's `last_selected` mutex.
async fn aggregate_plugin_tools(
    inner: &Arc<Inner>,
    result: &mut ListToolsResult,
    selection: &[PluginUpstreamKey],
    ws_session_id: Option<&str>,
) -> Result<(), ConduitError> {
    if let Some(ws_session_id) = ws_session_id {
        reconcile_interested_sessions(inner, selection, ws_session_id);
    }

    if selection.is_empty() {
        return Ok(());
    }

    let states = collect_plugin_states(inner, selection);
    let plugin_tool_lists: Vec<(PluginUpstreamKey, Arc<Vec<Tool>>)> =
        futures::future::try_join_all(states.into_iter().map(|(key, state)| async move {
            let tools = state
                .connection
                .list_tools()
                .await
                .map_err(|_| ConduitError::PluginListFailed)?;
            Ok::<_, ConduitError>((key, tools))
        }))
        .await?;

    let mut plugin_entries: Vec<Tool> = Vec::new();
    for ((_plugin, mcp_name, _base, _rid, _args), arc) in plugin_tool_lists {
        for tool in arc.iter() {
            let mut prefixed = tool.clone();
            prefixed.name = format!("{mcp_name}_{}", prefixed.name);
            plugin_entries.push(prefixed);
        }
    }

    merge_tools(&mut result.tools, plugin_entries);
    Ok(())
}

/// Aggregate resources from selected plugin MCP connections into the
/// primary upstream's `resources/list` response. Same shape as
/// [`aggregate_plugin_tools`] but operates on `result.resources` with
/// `uri` as the prefix-namespacing field. No name allow-list is
/// applied to primary resources (the agent declaration has no
/// `resources[]` field today — filtering is plugin-selection-only).
async fn aggregate_plugin_resources(
    inner: &Arc<Inner>,
    result: &mut ListResourcesResult,
    selection: &[PluginUpstreamKey],
    ws_session_id: Option<&str>,
) -> Result<(), ConduitError> {
    if let Some(ws_session_id) = ws_session_id {
        reconcile_interested_sessions(inner, selection, ws_session_id);
    }

    if selection.is_empty() {
        return Ok(());
    }

    let states = collect_plugin_states(inner, selection);
    let plugin_resource_lists: Vec<(
        PluginUpstreamKey,
        Arc<Vec<objectiveai_sdk::mcp::resource::Resource>>,
    )> = futures::future::try_join_all(states.into_iter().map(|(key, state)| async move {
        let resources = state
            .connection
            .list_resources()
            .await
            .map_err(|_| ConduitError::PluginListFailed)?;
        Ok::<_, ConduitError>((key, resources))
    }))
    .await?;

    let mut plugin_entries: Vec<objectiveai_sdk::mcp::resource::Resource> = Vec::new();
    for ((_plugin, mcp_name, _base, _rid, _args), arc) in plugin_resource_lists {
        for resource in arc.iter() {
            let mut prefixed = resource.clone();
            prefixed.uri = format!("{mcp_name}_{}", prefixed.uri);
            plugin_entries.push(prefixed);
        }
    }

    merge_resources(&mut result.resources, plugin_entries);
    Ok(())
}

/// Reconcile `interested_sessions` on each `PluginMcpState` against
/// the diff between this session's previous selection and the
/// current `mcp_servers` payload. Idempotent — called on every
/// `tools/list` and `resources/list` for the same ws_session_id.
fn reconcile_interested_sessions(
    inner: &Arc<Inner>,
    selection: &[PluginUpstreamKey],
    ws_session_id: &str,
) {
    let sess = get_or_create_session(inner, ws_session_id);
    let mut last = sess.last_selected.lock().unwrap();
    let new_set: HashSet<PluginUpstreamKey> = selection.iter().cloned().collect();
    let old_set: HashSet<PluginUpstreamKey> = last.iter().cloned().collect();
    for removed in old_set.difference(&new_set) {
        if let Some(state) = inner.plugin_mcp_connections.get(removed) {
            state.interested_sessions.remove(ws_session_id);
        }
    }
    for added in new_set.difference(&old_set) {
        if let Some(state) = inner.plugin_mcp_connections.get(added) {
            state.interested_sessions.insert(ws_session_id.to_string());
        }
    }
    *last = selection.to_vec();
}

/// Snapshot the `PluginMcpState`s for each upstream key the
/// selection references. Keys without a matching live connection
/// are silently skipped (the CONNECT never landed or got displaced)
/// — degraded but proceeds.
fn collect_plugin_states(
    inner: &Arc<Inner>,
    selection: &[PluginUpstreamKey],
) -> Vec<(PluginUpstreamKey, Arc<PluginMcpState>)> {
    selection
        .iter()
        .filter_map(|key| {
            inner
                .plugin_mcp_connections
                .get(key)
                .map(|s| (key.clone(), s.clone()))
        })
        .collect()
}

/// Merge `plugin_entries` into `primary_tools` with conflict
/// resolution on `name`: if any plugin tool's prefixed name
/// collides with a primary tool's name, prefix every primary tool
/// with `objectiveai-mcp_`. Then sort the merged list by `name` for
/// stable ordering.
fn merge_tools(primary_tools: &mut Vec<Tool>, plugin_entries: Vec<Tool>) {
    let plugin_names: HashSet<&str> = plugin_entries.iter().map(|t| t.name.as_str()).collect();
    let primary_collides = primary_tools
        .iter()
        .any(|t| plugin_names.contains(t.name.as_str()));
    if primary_collides {
        for tool in primary_tools.iter_mut() {
            tool.name = format!("objectiveai-mcp_{}", tool.name);
        }
    }
    primary_tools.extend(plugin_entries);
    primary_tools.sort_by(|a, b| a.name.cmp(&b.name));
}

/// Resource counterpart of [`merge_tools`] — merges on `uri`.
fn merge_resources(
    primary_resources: &mut Vec<objectiveai_sdk::mcp::resource::Resource>,
    plugin_entries: Vec<objectiveai_sdk::mcp::resource::Resource>,
) {
    let plugin_uris: HashSet<&str> = plugin_entries.iter().map(|r| r.uri.as_str()).collect();
    let primary_collides = primary_resources
        .iter()
        .any(|r| plugin_uris.contains(r.uri.as_str()));
    if primary_collides {
        for resource in primary_resources.iter_mut() {
            resource.uri = format!("objectiveai-mcp_{}", resource.uri);
        }
    }
    primary_resources.extend(plugin_entries);
    primary_resources.sort_by(|a, b| a.uri.cmp(&b.uri));
}

/// Try routing a `tools/call` to the upstream that exposes the
/// requested (prefixed) tool name. Fans out `Connection::list_tools`
/// across primary + selected plugins concurrently (all cached),
/// computes the primary's prefix from conflict-detection, finds the
/// matching upstream by stripping the prefix, rewrites `params.name`
/// to the unprefixed form, and forwards through the matching
/// connection. No match → typed `JsonRpcResult::Err(-32601)`.
async fn try_route_tools_call(
    inner: &Arc<Inner>,
    primary: Option<&ConduitState>,
    headers: &IndexMap<String, String>,
    config: &McpConfig,
    params: CallToolRequestParams,
) -> Result<server_response::Payload, ConduitError> {
    let requested = params.name.clone();

    let agent_instance_hierarchy = agent_instance_hierarchy_from_headers(headers);
    let response_id = response_id_from_headers(headers);
    let selection: Vec<PluginUpstreamKey> = config
        .mcp_servers
        .iter()
        .map(|e| e.cache_key(&agent_instance_hierarchy, &response_id))
        .collect();
    let states = collect_plugin_states(inner, &selection);
    let plugin_lists_fut =
        futures::future::try_join_all(states.into_iter().map(|(key, state)| async move {
            let tools = state
                .connection
                .list_tools()
                .await
                .map_err(|_| ConduitError::PluginListFailed)?;
            Ok::<_, ConduitError>((key, state, tools))
        }));
    let (primary_tools, plugin_lists) = match primary {
        Some(p) => {
            let primary_conn = &p.connection;
            let primary_tools_fut = async {
                primary_conn
                    .list_tools()
                    .await
                    .map_err(|_| ConduitError::PluginListFailed)
            };
            tokio::try_join!(primary_tools_fut, plugin_lists_fut)?
        }
        None => (Arc::new(Vec::new()), plugin_lists_fut.await?),
    };

    // Conflict detection: does any plugin's prefixed name match any
    // primary tool's name? (Trivially false if primary is None or
    // primary_tools is empty.)
    let plugin_prefixed: HashSet<String> = plugin_lists
        .iter()
        .flat_map(|((_p, mcp_name, _base, _rid, _args), _s, tools)| {
            let mcp_name = mcp_name.clone();
            tools
                .iter()
                .map(move |t| format!("{}_{}", mcp_name, t.name))
        })
        .collect();
    let primary_collides = primary_tools
        .iter()
        .any(|t| plugin_prefixed.contains(&t.name));
    let primary_prefix = if primary_collides {
        "objectiveai-mcp_"
    } else {
        ""
    };

    // Try primary first.
    if let Some(p) = primary {
        if let Some(stripped) = requested.strip_prefix(primary_prefix) {
            if primary_tools.iter().any(|t| t.name == stripped) {
                let mut routed_params = params.clone();
                routed_params.name = stripped.to_string();
                let result =
                    upstream_call::<CallToolRequestParams, CallToolResult>(
                        &p.connection,
                        headers,
                        "tools/call",
                        &routed_params,
                    )
                    .await?;
                return Ok(server_response::Payload::ToolsCall(result));
            }
        }
    }

    // Try each selected plugin.
    for ((_plugin, mcp_name, _base, _rid, _args), state, tools) in &plugin_lists {
        let prefix = format!("{mcp_name}_");
        if let Some(stripped) = requested.strip_prefix(&prefix) {
            if tools.iter().any(|t| t.name == stripped) {
                let mut routed_params = params.clone();
                routed_params.name = stripped.to_string();
                let result =
                    upstream_call::<CallToolRequestParams, CallToolResult>(
                        &state.connection,
                        headers,
                        "tools/call",
                        &routed_params,
                    )
                    .await?;
                return Ok(server_response::Payload::ToolsCall(result));
            }
        }
    }

    // No match anywhere.
    Ok(server_response::Payload::ToolsCall(JsonRpcResult::Err {
        code: -32601,
        message: format!("tool not found: {requested}"),
        data: None,
    }))
}

/// Try routing a `resources/read` to the upstream that exposes the
/// requested (prefixed) URI. Same shape as [`try_route_tools_call`]
/// but uses `list_resources` and `params.uri` / `resource.uri`.
async fn try_route_resources_read(
    inner: &Arc<Inner>,
    primary: Option<&ConduitState>,
    headers: &IndexMap<String, String>,
    config: &McpConfig,
    params: ReadResourceRequestParams,
) -> Result<server_response::Payload, ConduitError> {
    let requested = params.uri.clone();

    let agent_instance_hierarchy = agent_instance_hierarchy_from_headers(headers);
    let response_id = response_id_from_headers(headers);
    let selection: Vec<PluginUpstreamKey> = config
        .mcp_servers
        .iter()
        .map(|e| e.cache_key(&agent_instance_hierarchy, &response_id))
        .collect();
    let states = collect_plugin_states(inner, &selection);
    let plugin_lists_fut =
        futures::future::try_join_all(states.into_iter().map(|(key, state)| async move {
            let resources = state
                .connection
                .list_resources()
                .await
                .map_err(|_| ConduitError::PluginListFailed)?;
            Ok::<_, ConduitError>((key, state, resources))
        }));
    let (primary_resources, plugin_lists) = match primary {
        Some(p) => {
            let primary_conn = &p.connection;
            let primary_resources_fut = async {
                primary_conn
                    .list_resources()
                    .await
                    .map_err(|_| ConduitError::PluginListFailed)
            };
            tokio::try_join!(primary_resources_fut, plugin_lists_fut)?
        }
        None => (Arc::new(Vec::new()), plugin_lists_fut.await?),
    };

    let plugin_prefixed: HashSet<String> = plugin_lists
        .iter()
        .flat_map(|((_p, mcp_name, _base, _rid, _args), _s, resources)| {
            let mcp_name = mcp_name.clone();
            resources
                .iter()
                .map(move |r| format!("{}_{}", mcp_name, r.uri))
        })
        .collect();
    let primary_collides = primary_resources
        .iter()
        .any(|r| plugin_prefixed.contains(&r.uri));
    let primary_prefix = if primary_collides {
        "objectiveai-mcp_"
    } else {
        ""
    };

    if let Some(p) = primary {
        if let Some(stripped) = requested.strip_prefix(primary_prefix) {
            if primary_resources.iter().any(|r| r.uri == stripped) {
                let mut routed_params = params.clone();
                routed_params.uri = stripped.to_string();
                let result = upstream_call::<ReadResourceRequestParams, ReadResourceResult>(
                    &p.connection,
                    headers,
                    "resources/read",
                    &routed_params,
                )
                .await?;
                return Ok(server_response::Payload::ResourcesRead(result));
            }
        }
    }

    for ((_plugin, mcp_name, _base, _rid, _args), state, resources) in &plugin_lists {
        let prefix = format!("{mcp_name}_");
        if let Some(stripped) = requested.strip_prefix(&prefix) {
            if resources.iter().any(|r| r.uri == stripped) {
                let mut routed_params = params.clone();
                routed_params.uri = stripped.to_string();
                let result = upstream_call::<ReadResourceRequestParams, ReadResourceResult>(
                    &state.connection,
                    headers,
                    "resources/read",
                    &routed_params,
                )
                .await?;
                return Ok(server_response::Payload::ResourcesRead(result));
            }
        }
    }

    Ok(server_response::Payload::ResourcesRead(JsonRpcResult::Err {
        code: -32601,
        message: format!("resource not found: {requested}"),
        data: None,
    }))
}

/// Decoded `X-OBJECTIVEAI-MCP-CONFIG` payload. The JSON control
/// surface the API stamps on every request to the synthetic
/// `/objectiveai-mcp` URL — drives both tools/list filtering AND
/// plugin MCP server selection.
#[derive(Debug, serde::Deserialize)]
struct McpConfig {
    /// Allow-listed primary-upstream tool names. See
    /// [`apply_tools_filter`].
    #[serde(default)]
    names: Vec<String>,
    /// Whether objectiveai-mcp built-ins pass the filter. See
    /// [`apply_tools_filter`].
    #[serde(default)]
    objectiveai_builtins: bool,
    /// Per-`(plugin, name)` entries the server has chosen as active
    /// for this ws_session_id, plus each entry's `arguments` map.
    /// Drives `tools/list` aggregation across the primary upstream
    /// + selected plugin MCP connections and `list_changed`
    /// fan-out from selected plugin upstreams.
    #[serde(default)]
    mcp_servers: Vec<McpServerConfigEntry>,
}

/// One entry inside [`McpConfig::mcp_servers`]. Mirrors the API's
/// serialized `McpServerConfig` wire shape.
#[derive(Debug, Clone, serde::Deserialize)]
struct McpServerConfigEntry {
    plugin: String,
    name: String,
    #[serde(default)]
    arguments: Option<IndexMap<String, Option<String>>>,
}

impl McpServerConfigEntry {
    /// Canonical string form of the `arguments` map. Used as part
    /// of the [`Inner::plugin_mcp_connections`] cache key so two
    /// agents asking for the same `(plugin, name)` with different
    /// arguments don't share a connection. The map was already
    /// key-sorted by `prepare` on the SDK side; we just stringify it.
    fn args_canonical(&self) -> String {
        self.arguments
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default())
            .unwrap_or_default()
    }

    /// Cache key for `Inner::plugin_mcp_connections`. Both
    /// arguments come from the inbound request's headers
    /// (`X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` and `X-OBJECTIVEAI-RESPONSE-ID`).
    /// The per-agent-slot `response_id` is what actually enforces
    /// uniqueness; the base is carried alongside for diagnostic
    /// readability and wire-shape parity.
    fn cache_key(&self, agent_instance_hierarchy: &str, response_id: &str) -> PluginUpstreamKey {
        (
            self.plugin.clone(),
            self.name.clone(),
            agent_instance_hierarchy.to_string(),
            response_id.to_string(),
            self.args_canonical(),
        )
    }
}

/// Composite cache key for plugin-MCP upstream connections.
/// `(plugin_name, mcp_name, agent_instance_hierarchy, response_id,
/// args_canonical)`. Per-agent isolation comes from slot 3
/// (`response_id`), which the API mints fresh per agent slot —
/// guarantees distinct entries even if `agent_instance_hierarchy` ever
/// repeats. The streamed chunk's `agent_completion_ids()` emit
/// these exact response_ids, so the group-aware sweep in
/// [`ConduitMcpHandler::select_response_ids`] can match by slot 3
/// directly.
type PluginUpstreamKey = (String, String, String, String, String);

/// Case-insensitive `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` lookup on a request's
/// headers. Returns the empty string if the header is missing — that
/// case folds into a single shared cache slot for un-stamped
/// requests, matching the historical behavior before per-agent
/// isolation was added.
fn agent_instance_hierarchy_from_headers(headers: &IndexMap<String, String>) -> String {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// Case-insensitive `X-OBJECTIVEAI-AGENT-ID` lookup. Returns
/// the empty string on absence — the empty-string slot folds all
/// un-stamped requests together (mirrors `agent_instance_hierarchy_from_headers`),
/// which matches the historical default-everyone-shares slot.
fn agent_id_from_headers(headers: &IndexMap<String, String>) -> String {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-AGENT-ID"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// Case-insensitive `X-OBJECTIVEAI-RESPONSE-ID` lookup. The
/// API mints one of these per agent slot (one per `filtered_agents`
/// entry), so distinct slots always have distinct ids regardless
/// of any `agent_instance_hierarchy` collision. Used as the per-agent unique
/// component of [`PluginUpstreamKey`].
fn response_id_from_headers(headers: &IndexMap<String, String>) -> String {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-RESPONSE-ID"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// Parse `X-OBJECTIVEAI-RESPONSE-IDS` — the dash-joined sibling
/// group of response_ids the API minted for one agent completion.
/// Empty vec on absence; that's a no-op for the group-aware loser
/// sweep, so un-stamped requests degrade gracefully.
fn response_ids_group_from_headers(headers: &IndexMap<String, String>) -> Vec<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-RESPONSE-IDS"))
        .map(|(_, v)| v.split('-').map(str::to_owned).collect())
        .unwrap_or_default()
}

/// Register every response_id in this request's
/// `X-OBJECTIVEAI-RESPONSE-IDS` header under the same shared
/// `Arc<Vec<String>>` in [`Inner::response_id_groups`]. Cheap
/// (`Arc::clone` per id), idempotent under concurrent dials
/// (re-inserts of the same id overwrite with a logically-identical
/// Arc), no-op when the header is absent.
fn register_response_id_group(inner: &Arc<Inner>, headers: &IndexMap<String, String>) {
    let group = response_ids_group_from_headers(headers);
    if group.is_empty() {
        return;
    }
    let shared = Arc::new(group);
    for id in shared.iter() {
        inner
            .response_id_groups
            .insert(id.clone(), shared.clone());
    }
}

fn read_mcp_config_header(headers: &IndexMap<String, String>) -> Option<McpConfig> {
    use base64::Engine;
    let raw = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(MCP_CONFIG_HEADER))?
        .1;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw.as_bytes())
        .ok()?;
    serde_json::from_slice::<McpConfig>(&bytes).ok()
}

/// Filter a typed `ListToolsResult` in place. Keeps a returned tool
/// iff either:
///
/// - its name matches an explicit `allowed.names` entry exactly OR
///   with a `_<name>` suffix (mirrors the API's existing match
///   tolerance for upstream-namespaced tool names), OR
/// - `allowed.objectiveai_builtins` is set AND the tool's name isn't
///   among the CLI's locally-installed plugin/tool manifests (so it
///   must be an `objectiveai-mcp` built-in).
///
/// Drops everything else.
async fn apply_tools_filter(
    inner: &Arc<Inner>,
    result: &mut ListToolsResult,
    allowed: &McpConfig,
) {
    let installed: Option<&HashSet<String>> = if allowed.objectiveai_builtins {
        Some(
            inner
                .installed_names
                .get_or_init(|| load_installed_names(inner))
                .await,
        )
    } else {
        None
    };

    result.tools.retain(|tool| {
        if allowed
            .names
            .iter()
            .any(|declared| tool.name == *declared || tool.name.ends_with(&format!("_{declared}")))
        {
            return true;
        }
        if let Some(installed) = installed {
            return !installed.contains(&tool.name);
        }
        false
    });
}

/// Enumerate installed plugin + tool manifest names under
/// `config_base_dir`. Returns an empty set if the dir is unset or
/// neither directory exists. Used by the `objectiveai_builtins`
/// branch of [`apply_tools_filter`] to recognize built-ins by
/// elimination.
async fn load_installed_names(inner: &Arc<Inner>) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    let Some(base_dir) = inner.config_base_dir.clone() else {
        return names;
    };
    let fs =
        crate::filesystem::Client::new(Some(base_dir), None::<String>, None::<String>);
    for entry in fs.list_plugins(0, usize::MAX).await {
        names.insert(entry.name);
    }
    for entry in fs.list_tools(0, usize::MAX).await {
        names.insert(entry.name);
    }
    names
}

/// Extract the API↔CLI routing identifier (the `ws_session_id`) the
/// agent client stamps on every proxy-forwarded request.
fn ws_session_id_from_headers(headers: &IndexMap<String, String>) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-RESPONSE-ID"))
        .map(|(_, v)| v.clone())
}

/// Get-or-create the per-ws_session_id [`SessionState`]. Lazy: the
/// first request that carries an `X-OBJECTIVEAI-RESPONSE-ID` for a
/// given id materialises the entry; subsequent calls return the
/// same `Arc`.
fn get_or_create_session(inner: &Arc<Inner>, ws_session_id: &str) -> Arc<SessionState> {
    inner
        .sessions
        .entry(ws_session_id.to_string())
        .or_insert_with(|| {
            Arc::new(SessionState {
                last_selected: std::sync::Mutex::new(Vec::new()),
                mcp_session_id: OnceLock::new(),
            })
        })
        .clone()
}

/// Parses bare JSON; falls back to stripping `data:` prefixes and
/// reparsing for SSE-wrapped responses.
fn parse_json_or_sse(text: &str) -> Option<serde_json::Value> {
    if text.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return Some(v);
    }
    let collected: String = text
        .lines()
        .filter_map(|l| l.strip_prefix("data: ").or_else(|| l.strip_prefix("data:")))
        .collect();
    if collected.is_empty() {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(&collected).ok()
}

#[derive(Debug, thiserror::Error)]
enum ConduitError {
    #[error("forwarding HTTP request failed: {0}")]
    Request(reqwest::Error),
    #[error("reading response body failed: {0}")]
    Body(reqwest::Error),
    #[error("upstream response was malformed: {0}")]
    MalformedUpstream(String),
    #[error("plugin upstream list_tools / list_resources failed")]
    PluginListFailed,
    #[error("primary upstream connect failed")]
    PrimaryDialFailed,
    #[error("plugin upstream {plugin_name:?}/{mcp_name:?} dial failed: {reason}")]
    PluginDialFailed {
        plugin_name: String,
        mcp_name: String,
        reason: String,
    },
}

/// Aggregate session id returned to the proxy on `initialize`.
/// Encodes every upstream's individual `Mcp-Session-Id` plus the
/// `(plugin_name, mcp_name)` identity for each plugin connection so
/// subsequent requests can route to the right `PluginMcpState` and
/// (future commit) reconnects can resume each upstream with its
/// original session id via `Client::connect(url, Some(sid), …)`.
///
/// Wire format: JSON-serialize → base62-encode (plain — no AEAD,
/// since CLI ↔ API runs over a trusted WS). Modeled on
/// `objectiveai-mcp-proxy/src/session_manager.rs` minus the
/// encryption.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct AggregateSession {
    /// Primary upstream's `Mcp-Session-Id` (the local objectiveai-mcp
    /// HTTP server). `None` when the agent didn't need primary
    /// (`names` empty AND `objectiveai_builtins=false`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    primary: Option<String>,
    /// Per-selected-plugin entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    plugins: Vec<AggregatePluginEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AggregatePluginEntry {
    plugin_name: String,
    mcp_name: String,
    mcp_session_id: String,
}

impl AggregateSession {
    fn encode(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("AggregateSession serializes");
        base62_encode_bytes(&bytes)
    }

    fn decode(s: &str) -> Option<Self> {
        let bytes = base62_decode_bytes(s)?;
        serde_json::from_slice(&bytes).ok()
    }
}

/// Byte-level base62. Lifted from
/// `objectiveai-mcp-proxy/src/session_manager.rs:317-348` — the
/// off-the-shelf `base62` crate only encodes `u128`s. Interprets the
/// bytes as a big-endian unsigned big-integer; leading zero bytes
/// are encoded as `0` digits so they survive the round-trip.
fn base62_encode_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    const ALPHABET: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let leading_zeros = bytes.iter().take_while(|b| **b == 0).count();
    let mut digits: Vec<u8> = Vec::with_capacity(bytes.len() * 2);
    let mut num: Vec<u32> = bytes[leading_zeros..].iter().map(|b| *b as u32).collect();
    while !num.is_empty() {
        let mut remainder: u32 = 0;
        let mut next: Vec<u32> = Vec::with_capacity(num.len());
        for &b in &num {
            let acc = remainder * 256 + b;
            let q = acc / 62;
            remainder = acc % 62;
            if !(next.is_empty() && q == 0) {
                next.push(q);
            }
        }
        digits.push(remainder as u8);
        num = next;
    }
    let mut out = String::with_capacity(leading_zeros + digits.len());
    for _ in 0..leading_zeros {
        out.push(ALPHABET[0] as char);
    }
    for d in digits.into_iter().rev() {
        out.push(ALPHABET[d as usize] as char);
    }
    out
}

fn base62_decode_bytes(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    fn digit(c: char) -> Option<u32> {
        match c {
            '0'..='9' => Some(c as u32 - '0' as u32),
            'a'..='z' => Some(c as u32 - 'a' as u32 + 10),
            'A'..='Z' => Some(c as u32 - 'A' as u32 + 36),
            _ => None,
        }
    }
    let leading_zeros = s.chars().take_while(|c| *c == '0').count();
    let mut num: Vec<u32> = Vec::with_capacity(s.len());
    for c in s.chars().skip(leading_zeros) {
        num.push(digit(c)?);
    }
    let mut bytes: Vec<u8> = Vec::new();
    while !num.is_empty() {
        let mut remainder: u32 = 0;
        let mut next: Vec<u32> = Vec::with_capacity(num.len());
        for &d in &num {
            let acc = remainder * 62 + d;
            let q = acc / 256;
            remainder = acc % 256;
            if !(next.is_empty() && q == 0) {
                next.push(q);
            }
        }
        bytes.push(remainder as u8);
        num = next;
    }
    let mut out = vec![0u8; leading_zeros];
    out.extend(bytes.into_iter().rev());
    Some(out)
}

#[cfg(test)]
mod aggregate_tests {
    use super::*;

    #[test]
    fn aggregate_round_trip() {
        let agg = AggregateSession {
            primary: Some("primary-session-abc".into()),
            plugins: vec![
                AggregatePluginEntry {
                    plugin_name: "p1".into(),
                    mcp_name: "m1".into(),
                    mcp_session_id: "plug-1".into(),
                },
                AggregatePluginEntry {
                    plugin_name: "p2".into(),
                    mcp_name: "m2".into(),
                    mcp_session_id: "plug-2".into(),
                },
            ],
        };
        let encoded = agg.encode();
        let decoded = AggregateSession::decode(&encoded).expect("decode");
        assert_eq!(decoded.primary.as_deref(), Some("primary-session-abc"));
        assert_eq!(decoded.plugins.len(), 2);
        assert_eq!(decoded.plugins[0].plugin_name, "p1");
        assert_eq!(decoded.plugins[1].mcp_session_id, "plug-2");
    }

    #[test]
    fn aggregate_empty_primary() {
        let agg = AggregateSession {
            primary: None,
            plugins: vec![AggregatePluginEntry {
                plugin_name: "p".into(),
                mcp_name: "m".into(),
                mcp_session_id: "s".into(),
            }],
        };
        let encoded = agg.encode();
        let decoded = AggregateSession::decode(&encoded).expect("decode");
        assert!(decoded.primary.is_none());
        assert_eq!(decoded.plugins.len(), 1);
    }

    #[test]
    fn base62_round_trip_samples() {
        let samples: Vec<&[u8]> = vec![b"hello", b"\x00\x00abc", &[0u8; 8], &[255u8; 16], b""];
        for s in samples {
            let encoded = base62_encode_bytes(s);
            let decoded = base62_decode_bytes(&encoded).expect("decode");
            assert_eq!(decoded.as_slice(), s);
        }
    }
}
