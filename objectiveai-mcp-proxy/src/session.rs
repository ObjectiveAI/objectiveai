//! Per-session state and per-session dispatch.
//!
//! A `Session` owns the upstream MCP connections that belong to one MCP
//! session and is responsible for fanning `tools/list` / `resources/list`
//! out to them and routing `tools/call` / `resources/read` to the right
//! upstream. The registry that minds session ids and hands out
//! `Arc<Session>`s lives in [`crate::session_manager`].

use dashmap::DashMap;
use futures::future::join_all;
use indexmap::IndexMap;
use objectiveai::mcp::{
    Connection, JsonRpcNotification,
    resource::{ListResourcesResult, ReadResourceResult, Resource},
    tool::{CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult, Tool},
};
use tokio::sync::{Mutex, broadcast};
use tokio_util::sync::CancellationToken;

/// Capacity of the per-session outbound notification channel. Sized so
/// even a noisy upstream can't easily lap a slow SSE consumer.
const OUTBOUND_CAPACITY: usize = 64;

/// Hashable key for a JSON-RPC request id. JSON-RPC ids can be number,
/// string, or null, so we serialize to canonical JSON and hash on that.
fn request_id_key(id: &serde_json::Value) -> String {
    // serde_json::to_string is infallible for any Value.
    serde_json::to_string(id).unwrap_or_default()
}

/// Per-session state.
///
/// All routing, fan-out, and forwarding methods live here. The registry
/// that hands out `Arc<Session>`s by id is `SessionManager` (in
/// [`crate::session_manager`]).
#[derive(Debug)]
pub struct Session {
    /// Live upstream MCP connections keyed by their
    /// `initialize_result.server_info.name`. The key is the same string the
    /// proxy uses as the `<server-name>_` prefix on every tool name and
    /// resource URI it ships, so routing inbound `tools/call` /
    /// `resources/read` is just a longest-prefix-match lookup against this
    /// map's keys — no side-channel cache to keep coherent.
    ///
    /// Insertion order matches the order URLs appeared in `X-MCP-Servers`,
    /// so listings are deterministic.
    ///
    /// `Connection` is itself a cheaply-clonable Arc wrapper; dropping a
    /// `Connection` fires the upstream listener's wakeup signal so it can
    /// self-cancel within scheduler latency once no external handle remains.
    pub connections: IndexMap<String, Connection>,
    /// Fan-out channel for server-initiated notifications. Whenever an
    /// upstream emits `notifications/tools/list_changed` or
    /// `notifications/resources/list_changed`, a JsonRpcNotification with
    /// the matching method is published here. Subscribers (the SSE GET
    /// stream in `mcp::handle_get`) drain it onto the wire to the
    /// downstream client.
    ///
    /// `broadcast` rather than `mpsc` so multiple concurrent GET streams
    /// for the same session — which the MCP spec allows — each see every
    /// notification.
    pub outbound: broadcast::Sender<JsonRpcNotification>,
    /// In-flight per-request cancellation tokens, keyed by the inbound
    /// JSON-RPC request id (stringified for hashability — JSON-RPC ids
    /// can be number, string, or null). The downstream client cancels a
    /// request by sending `notifications/cancelled` with the matching
    /// `requestId`; the handler that owns that id observes the token
    /// firing via `tokio::select!` and returns a `-32800 request cancelled`
    /// JSON-RPC error. Drops the upstream call's future as a side effect.
    in_flight: DashMap<String, CancellationToken>,
    /// Content blocks accumulated via `POST /notify` between tool calls.
    /// Drained and prepended (wrapped in a `<system-reminder>` text
    /// block pair) on the next `tools/call` response so the model picks
    /// the message up at its next natural inspection point.
    pending_notifications: Mutex<Vec<ContentBlock>>,
}

impl Session {
    pub(crate) fn new(connections: IndexMap<String, Connection>) -> Self {
        let (outbound, _) = broadcast::channel(OUTBOUND_CAPACITY);

        // Wire each upstream's list_changed callbacks to publish a
        // matching notification onto the outbound channel. Callbacks fire
        // under the upstream's cache write lock (before the network
        // refresh), so by the time the downstream client re-fetches via
        // tools/list or resources/list it'll either get the new list or
        // wait on the lock until the new list lands — never sees the
        // stale list with a fresh notification.
        for connection in connections.values() {
            let tx = outbound.clone();
            connection.set_on_tools_list_changed(move || {
                let _ = tx.send(JsonRpcNotification {
                    jsonrpc: "2.0".into(),
                    method: "notifications/tools/list_changed".into(),
                    params: None,
                });
            });
            let tx = outbound.clone();
            connection.set_on_resources_list_changed(move || {
                let _ = tx.send(JsonRpcNotification {
                    jsonrpc: "2.0".into(),
                    method: "notifications/resources/list_changed".into(),
                    params: None,
                });
            });
        }

        Self {
            connections,
            outbound,
            in_flight: DashMap::new(),
            pending_notifications: Mutex::new(Vec::new()),
        }
    }

    /// Append `blocks` to the pending-notifications queue. The next
    /// `tools/call` response on this session drains and prepends them.
    pub async fn enqueue_notifications(&self, blocks: Vec<ContentBlock>) {
        if blocks.is_empty() {
            return;
        }
        self.pending_notifications.lock().await.extend(blocks);
    }

    /// Atomically take the queued notifications. Subsequent calls return
    /// `Vec::new()` until the next enqueue.
    pub async fn drain_notifications(&self) -> Vec<ContentBlock> {
        std::mem::take(&mut *self.pending_notifications.lock().await)
    }

    /// Mint a [`CancellationToken`] for an inbound request id, store it,
    /// and hand back a clone. The handler `select!`s on the clone; the
    /// stored token is what `cancel_in_flight` fires.
    pub fn register_in_flight(&self, id: &serde_json::Value) -> CancellationToken {
        let token = CancellationToken::new();
        self.in_flight.insert(request_id_key(id), token.clone());
        token
    }

    /// Drop the in-flight token for `id`. Always paired with an earlier
    /// `register_in_flight` via a guard so we don't leak entries on the
    /// happy path.
    pub fn deregister_in_flight(&self, id: &serde_json::Value) {
        self.in_flight.remove(&request_id_key(id));
    }

    /// Fire the cancellation token associated with `id`, if any. Returns
    /// `true` if a token was found and cancelled. Triggered by an inbound
    /// `notifications/cancelled` from the downstream client.
    pub fn cancel_in_flight(&self, id: &serde_json::Value) -> bool {
        match self.in_flight.get(&request_id_key(id)) {
            Some(entry) => {
                entry.value().cancel();
                true
            }
            None => false,
        }
    }

    /// Fan `tools/list` out to every upstream in parallel, prefix each
    /// tool's name with `<server-name>_`, concatenate the per-upstream
    /// lists, and return the union. Per-upstream failures are logged and
    /// the upstream is dropped from the result — one bad server can't
    /// poison the whole listing.
    pub async fn list_tools(&self) -> ListToolsResult {
        let names: Vec<&String> = self.connections.keys().collect();
        let results = join_all(
            self.connections
                .values()
                .map(|c| async move { c.list_tools().await }),
        )
        .await;

        let mut tools: Vec<Tool> = Vec::new();
        for (server_name, result) in names.into_iter().zip(results) {
            match result {
                Ok(arc) => {
                    for tool in arc.iter() {
                        let mut prefixed = tool.clone();
                        prefixed.name = prefix_name(server_name, &tool.name);
                        tools.push(prefixed);
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, upstream = %server_name, "list_tools failed")
                }
            }
        }

        ListToolsResult {
            tools,
            next_cursor: None,
            _meta: None,
        }
    }

    /// Fan `resources/list` out to every upstream in parallel, prefix each
    /// URI with `<server-name>_`, concatenate the per-upstream lists, and
    /// return the union. Same best-effort failure semantics as
    /// [`Session::list_tools`].
    pub async fn list_resources(&self) -> ListResourcesResult {
        let names: Vec<&String> = self.connections.keys().collect();
        let results = join_all(
            self.connections
                .values()
                .map(|c| async move { c.list_resources().await }),
        )
        .await;

        let mut resources: Vec<Resource> = Vec::new();
        for (server_name, result) in names.into_iter().zip(results) {
            match result {
                Ok(arc) => {
                    for resource in arc.iter() {
                        let mut prefixed = resource.clone();
                        prefixed.uri = prefix_name(server_name, &resource.uri);
                        resources.push(prefixed);
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, upstream = %server_name, "list_resources failed")
                }
            }
        }

        ListResourcesResult {
            resources,
            next_cursor: None,
            _meta: None,
        }
    }

    /// Forward `tools/call` to whichever upstream owns the named tool.
    /// Routing is longest-prefix-match against the connection map's keys —
    /// see [`Session::route`].
    pub async fn call_tool(
        &self,
        params: &CallToolRequestParams,
    ) -> Result<CallToolResult, CallToolError> {
        let (connection, original_name) = self
            .route(&params.name)
            .ok_or_else(|| CallToolError::ToolNotFound(params.name.clone()))?;

        // Forward to the upstream with the un-prefixed tool name it actually
        // knows; pass everything else (`arguments`, `task`, `_meta`) through
        // unchanged.
        let upstream_params = CallToolRequestParams {
            name: original_name,
            arguments: params.arguments.clone(),
            task: params.task.clone(),
            _meta: params._meta.clone(),
        };
        Ok(connection.call_tool(&upstream_params).await?)
    }

    /// Forward `resources/read` to whichever upstream owns the URI. Same
    /// longest-prefix-match routing as [`Session::call_tool`].
    pub async fn read_resource(
        &self,
        uri: &str,
    ) -> Result<ReadResourceResult, ReadResourceError> {
        let (connection, original_uri) = self
            .route(uri)
            .ok_or_else(|| ReadResourceError::ResourceNotFound(uri.to_string()))?;
        Ok(connection.read_resource(&original_uri).await?)
    }

    /// Resolve a `<server-name>_<original>` prefixed identifier to the
    /// owning connection and the original (un-prefixed) name the upstream
    /// actually knows.
    ///
    /// Server names that contain `_` are supported via longest-prefix
    /// match: if both `fs` and `fs_extra` are connected and the inbound
    /// name is `fs_extra_Read`, the `fs_extra` upstream wins.
    fn route<'a>(&'a self, prefixed: &str) -> Option<(&'a Connection, String)> {
        let mut best: Option<(&'a str, &'a Connection)> = None;
        for (name, conn) in &self.connections {
            // Need at least one char after the `_` to count as a real prefix
            // hit (otherwise an exact match `name == prefixed` would route
            // to an empty original name).
            if prefixed.len() > name.len() + 1
                && prefixed.as_bytes()[name.len()] == b'_'
                && prefixed.starts_with(name.as_str())
            {
                if best.map(|(b, _)| name.len() > b.len()).unwrap_or(true) {
                    best = Some((name.as_str(), conn));
                }
            }
        }
        best.map(|(name, conn)| {
            let original = prefixed[name.len() + 1..].to_string();
            (conn, original)
        })
    }
}

/// Prefix a tool name or resource URI with the upstream server name.
/// Format: `<server-name>_<original>`.
fn prefix_name(server_name: &str, name: &str) -> String {
    format!("{server_name}_{name}")
}

/// Failure modes for [`Session::call_tool`].
#[derive(Debug, thiserror::Error)]
pub enum CallToolError {
    #[error("tool not found on any upstream: {0}")]
    ToolNotFound(String),
    #[error("upstream call_tool failed: {0}")]
    Upstream(#[from] objectiveai::mcp::Error),
}

/// Failure modes for [`Session::read_resource`].
#[derive(Debug, thiserror::Error)]
pub enum ReadResourceError {
    #[error("resource not found on any upstream: {0}")]
    ResourceNotFound(String),
    #[error("upstream read_resource failed: {0}")]
    Upstream(#[from] objectiveai::mcp::Error),
}

