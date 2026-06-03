//! Per-session state and per-session dispatch.
//!
//! A `Session` owns the upstream MCP connections that belong to one MCP
//! session and is responsible for fanning `tools/list` / `resources/list`
//! out to them and routing `tools/call` / `resources/read` to the right
//! upstream. The registry that minds session ids and hands out
//! `Arc<Session>`s lives in [`crate::session_manager`].

use dashmap::DashMap;
use futures::future::try_join_all;
use indexmap::IndexMap;
use std::sync::Arc;
use objectiveai_sdk::mcp::{
    Connection, JsonRpcNotification,
    resource::{ListResourcesResult, ReadResourceResult, Resource},
    tool::{CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult, Tool},
};
use axum::http::HeaderMap;
use tokio::sync::{Mutex, RwLock, broadcast};
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
    /// The canonical `URL → header_map` payload that was encoded into
    /// this session's id. Used by `handle_initialize`'s
    /// alive-in-memory branch to re-mint an id from the same byte-
    /// stable shape that was originally encoded — so even if the live
    /// `Connection`s rotated their internal state, the id remains
    /// derivable from the immutable per-upstream header set.
    pub payload: crate::session_manager::SessionPayload,
    /// Session-global header overrides stamped on every outbound
    /// request to every upstream. Lives only in memory (NOT encoded
    /// into the session id) so it doesn't survive proxy restart. The
    /// only keys ever recorded are `X-OBJECTIVEAI-RESPONSE-ID` and
    /// `X-OBJECTIVEAI-RESPONSE-IDS` — extracted from inbound
    /// `initialize` HeaderMaps by [`Self::apply_transient_headers`].
    ///
    /// Reads dominate writes: every outbound request through any
    /// upstream Connection reads (via [`Connection::set_extra_headers`]
    /// applied on the Connection's own RwLock); writes fire only on
    /// inbound `initialize` (alive, decrypt+reconnect, fresh).
    /// `RwLock` matches the read/write ratio.
    pub transient_headers: RwLock<IndexMap<String, String>>,
}

impl Session {
    pub(crate) fn new(
        connections: IndexMap<String, Connection>,
        payload: crate::session_manager::SessionPayload,
    ) -> Self {
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
            payload,
            transient_headers: RwLock::new(IndexMap::new()),
        }
    }

    /// Header keys persisted on the session-global transient bag —
    /// extracted from inbound `initialize` headers and stamped on
    /// every outbound upstream request via
    /// [`Connection::set_extra_headers`]. Closed set; not extensible.
    pub const TRANSIENT_HEADER_KEYS: [&'static str; 2] = [
        "X-OBJECTIVEAI-RESPONSE-ID",
        "X-OBJECTIVEAI-RESPONSE-IDS",
    ];

    /// Build the transient bag from an inbound `initialize` HeaderMap,
    /// FULL-REPLACE [`Self::transient_headers`] with it, then fan the
    /// new bag onto every upstream `Connection`'s `extra_headers` via
    /// [`Connection::set_extra_headers`].
    ///
    /// Missing keys in `src` → absent from the new bag → dropped from
    /// every Connection's `extra_headers` too. Never merges with the
    /// previous bag; this is the "replace, even if missing some" rule.
    pub async fn apply_transient_headers(&self, src: &HeaderMap) {
        let mut bag = IndexMap::new();
        for key in Self::TRANSIENT_HEADER_KEYS {
            if let Some(v) = src.get(key).and_then(|v| v.to_str().ok()) {
                bag.insert(key.to_string(), v.to_string());
            }
        }
        let snapshot = bag.clone();
        *self.transient_headers.write().await = bag;
        for connection in self.connections.values() {
            connection.set_extra_headers(snapshot.clone()).await;
        }
    }

    /// Append `blocks` to the pending-notifications queue. The next
    /// `tools/call` response on this session drains and prepends them.
    /// Caller-supplied `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` captured at session-
    /// open time. Recovered from the encrypted session-id payload on
    /// every resume, so upstream sdk runners (which only carry the
    /// session id back) never have to re-send the header themselves.
    pub fn agent_instance_hierarchy(&self) -> Option<&str> {
        self.payload.agent_instance_hierarchy.as_deref()
    }

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

    /// Non-draining peek — `true` iff the pending-notifications queue
    /// holds at least one block. Companion to [`Self::drain_notifications`]
    /// for callers that want to know whether a drain *would* return
    /// anything without consuming the queue.
    pub async fn has_pending_notifications(&self) -> bool {
        !self.pending_notifications.lock().await.is_empty()
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
    /// lists, and return the union sorted by name. Fails fast: the
    /// first upstream error short-circuits via `try_join_all` and is
    /// returned to the caller — we don't paper over a broken upstream.
    ///
    /// Sorting by name guarantees a stable order across calls regardless
    /// of upstream `HashMap` iteration order or per-upstream return
    /// order; downstream consumers (e.g. seeded mock agents) rely on
    /// this for deterministic output.
    pub async fn list_tools(&self) -> Result<ListToolsResult, Arc<objectiveai_sdk::mcp::Error>> {
        self.list_tools_filtered(None).await
    }

    /// Per-upstream variant of [`Self::list_tools`]: when `filter_url`
    /// is `Some`, only fans out to the single upstream whose connection
    /// `url` matches verbatim. When `None`, behaves identically to the
    /// no-arg form (fan out to every upstream).
    ///
    /// An unmatched `filter_url` returns an empty `ListToolsResult`
    /// (not an error) — the caller validates whether emptiness is
    /// acceptable. The per-upstream allowlist (`X-MCP-Tools-Allow`)
    /// is still applied to whichever upstream(s) participate.
    pub async fn list_tools_filtered(
        &self,
        filter_url: Option<&str>,
    ) -> Result<ListToolsResult, Arc<objectiveai_sdk::mcp::Error>> {
        let pairs: Vec<(&String, &Connection)> = match filter_url {
            Some(url) => self
                .connections
                .iter()
                .filter(|(_, c)| c.url == url)
                .collect(),
            None => self.connections.iter().collect(),
        };
        let results = try_join_all(
            pairs
                .iter()
                .map(|(_, c)| async move {
                    let r = c.list_tools().await;
                    r
                }),
        )
        .await?;

        let mut tools: Vec<Tool> = Vec::new();
        for ((server_name, _), arc) in pairs.into_iter().zip(results) {
            for tool in arc.iter() {
                let mut prefixed = tool.clone();
                prefixed.name = prefix_name(server_name, &tool.name);
                tools.push(prefixed);
            }
        }
        tools.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            _meta: None,
        })
    }

    /// Fan `resources/list` out to every upstream in parallel, prefix
    /// each URI with `<server-name>_`, concatenate the per-upstream
    /// lists, and return the union sorted by URI. Same fail-fast
    /// semantics as [`Session::list_tools`] — the first upstream error
    /// short-circuits and is returned to the caller.
    pub async fn list_resources(&self) -> Result<ListResourcesResult, Arc<objectiveai_sdk::mcp::Error>> {
        self.list_resources_filtered(None).await
    }

    /// Per-upstream variant of [`Self::list_resources`]: when
    /// `filter_url` is `Some`, only fans out to the single upstream
    /// whose connection `url` matches verbatim. When `None`, behaves
    /// identically to the no-arg form (fan out to every upstream).
    ///
    /// An unmatched `filter_url` returns an empty `ListResourcesResult`
    /// (not an error) — the caller validates whether emptiness is
    /// acceptable.
    pub async fn list_resources_filtered(
        &self,
        filter_url: Option<&str>,
    ) -> Result<ListResourcesResult, Arc<objectiveai_sdk::mcp::Error>> {
        let pairs: Vec<(&String, &Connection)> = match filter_url {
            Some(url) => self
                .connections
                .iter()
                .filter(|(_, c)| c.url == url)
                .collect(),
            None => self.connections.iter().collect(),
        };
        let results = try_join_all(
            pairs
                .iter()
                .map(|(_, c)| async move {
                    let r = c.list_resources().await;
                    r
                }),
        )
        .await?;

        let mut resources: Vec<Resource> = Vec::new();
        for ((server_name, _), arc) in pairs.into_iter().zip(results) {
            for resource in arc.iter() {
                let mut prefixed = resource.clone();
                prefixed.uri = prefix_name(server_name, &resource.uri);
                resources.push(prefixed);
            }
        }
        resources.sort_by(|a, b| a.uri.cmp(&b.uri));

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            _meta: None,
        })
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
        let r = connection.call_tool(&upstream_params).await;
        Ok(r?)
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
        let r = connection.read_resource(&original_uri).await;
        Ok(r?)
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
    Upstream(#[from] objectiveai_sdk::mcp::Error),
}

/// Failure modes for [`Session::read_resource`].
#[derive(Debug, thiserror::Error)]
pub enum ReadResourceError {
    #[error("resource not found on any upstream: {0}")]
    ResourceNotFound(String),
    #[error("upstream read_resource failed: {0}")]
    Upstream(#[from] objectiveai_sdk::mcp::Error),
}

