//! Reverse-attach registry + handle types.
//!
//! Lets the API's `/objectiveai-mcp` route forward a proxy's request
//! as a `server_request::Request` over the matching in-flight
//! WebSocket. Used by the JSON-RPC handlers in [`super::handlers`]
//! and registered against by every `_ws` upgrade handler in
//! `streaming_ws_handlers`.

use super::listeners::McpListenerRegistry;
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures::stream::SplitSink;
use objectiveai_sdk::client_objectiveai_mcp::server_response;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};

/// Shared sender half of a split WebSocket, wrapped under a tokio
/// mutex so the send-side (chunk forwarder) and recv-side (notify
/// responder + server_request emitter) can both write frames safely.
/// Locks are short-lived — only held across a single `send`.
pub type SharedSink = Arc<Mutex<SplitSink<WebSocket, Message>>>;

/// Per-WS-connection tracker of agent-completion `response_id`s
/// observed on the send side. The recv loop consults it to reject
/// `AgentCompletionNotify` requests targeting an id not produced by
/// this stream.
pub struct SessionTracker {
    ids: dashmap::DashSet<String>,
}

impl SessionTracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            ids: dashmap::DashSet::new(),
        })
    }

    /// Extend the tracker with every agent-completion id this chunk
    /// carries. Borrows into the chunk; no allocation beyond the
    /// `insert` itself.
    pub fn observe<C>(&self, chunk: &C)
    where
        C: objectiveai_sdk::agent::completions::response::streaming::AgentCompletionIds,
    {
        for id in chunk.agent_completion_ids() {
            self.ids.insert(id.to_string());
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }
}

/// Per-WS-connection registry of outstanding
/// [`server_request::Request`](objectiveai_sdk::client_objectiveai_mcp::server_request::Request)s
/// the API has emitted and is awaiting a matching
/// [`server_response::Response`] for. Keys are the API-minted `id`;
/// values are the oneshot the awaiting future is parked on. The recv
/// side of the WS drains `server_response` frames, looks up `id`, and
/// fulfills the oneshot with the full response.
pub type PendingRequests = Arc<DashMap<String, oneshot::Sender<server_response::Response>>>;

pub fn new_pending_requests() -> PendingRequests {
    Arc::new(DashMap::new())
}

/// Reverse-attach handle for the API's MCP endpoint to forward proxy
/// traffic over an in-flight WS. Holds both halves of the per-
/// connection state: the sink to write `server_request` frames out,
/// and the registry to park awaits for matching `server_response`
/// frames coming back.
#[derive(Clone)]
pub struct ReverseChannel {
    pub sink: SharedSink,
    pub pending: PendingRequests,
}

/// Process-wide registry of live [`ReverseChannel`]s keyed by the
/// per-agent `response_id` registered on WS upgrade. Populated by
/// the per-endpoint `_ws` handlers; consulted by the per-MCP routes
/// (`/objectiveai` and `/{owner}/{name}/{version}/{mcp}`) — both
/// look up by the `X-OBJECTIVEAI-RESPONSE-ID` header — and by the
/// agent-completion verification probe.
pub type ReverseChannelRegistry = Arc<DashMap<String, ReverseChannel>>;

pub fn new_reverse_channel_registry() -> ReverseChannelRegistry {
    Arc::new(DashMap::new())
}

/// Bundle of the things each `_ws` handler needs to wire up the
/// reverse-attach:
///
/// - [`ReverseChannelRegistry`] so the handler can insert/remove its
///   session.
/// - `mcp_port` — the API's loopback-only MCP listener port. The
///   agent client uses it to build synthetic
///   `http://127.0.0.1:<mcp_port>/objectiveai` and
///   `http://127.0.0.1:<mcp_port>/{owner}/{name}/{ver}/{mcp}` URLs
///   that the proxy will dial. Kernel-enforced: the listener binds
///   `127.0.0.1` so non-loopback callers cannot reach it.
/// - [`McpListenerRegistry`] so the recv loop's `McpListChanged`
///   dispatch can publish to the per-(response_id, McpKind)
///   broadcast feeding the API's GET-SSE notifications stream.
#[derive(Clone)]
pub struct ReverseAttachConfig {
    pub registry: ReverseChannelRegistry,
    pub mcp_port: u16,
    pub mcp_listeners: McpListenerRegistry,
}

/// Arc-shareable handle the agent client uses to register per-agent
/// `response_id`s against the current WS [`ReverseChannel`]. Many
/// ids may map to one channel — one CLI WS upgrade can serve a swarm
/// of N agents, each declaring `client_objectiveai_mcp` with its own
/// per-turn `response_id`. The owning [`ReverseAttachGuard`] removes
/// every registered id on drop.
pub struct ReverseAttachHandle {
    registry: ReverseChannelRegistry,
    channel: ReverseChannel,
    registered: std::sync::Mutex<Vec<String>>,
}

impl std::fmt::Debug for ReverseAttachHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self
            .registered
            .try_lock()
            .map(|g| g.len())
            .unwrap_or(usize::MAX);
        f.debug_struct("ReverseAttachHandle")
            .field("registered_count", &count)
            .finish_non_exhaustive()
    }
}

impl ReverseAttachHandle {
    /// Inserts `id -> channel` into the registry and tracks the id
    /// for cleanup when the owning guard drops. Calling with the same
    /// id twice is harmless (the registry just overwrites).
    pub fn register(&self, id: String) {
        self.registry.insert(id.clone(), self.channel.clone());
        self.registered.lock().unwrap().push(id);
    }

    /// Snapshot of every id this handle has registered so far. Used
    /// by the conduit's list-changed dispatcher to fan one
    /// `McpListChanged` out to every matching SSE subscriber on this
    /// WS.
    pub fn registered_ids(&self) -> Vec<String> {
        self.registered.lock().unwrap().clone()
    }

    /// The WS reverse channel this upgrade registered against. Every
    /// per-agent `response_id` registered through this handle
    /// resolves to the same underlying channel — callers that need
    /// to send `server_request` frames (e.g. plugin-MCP-begin) reach
    /// the sink + pending registry through here without going through
    /// the full `ReverseChannelRegistry` lookup.
    pub fn channel(&self) -> &ReverseChannel {
        &self.channel
    }
}

/// RAII guard for one CLI WS upgrade. Owns the registration handle;
/// when it drops, every id registered via the handle is removed from
/// the [`ReverseChannelRegistry`]. `Arc` clones of the handle may
/// outlive the guard (e.g. background tasks holding onto a copy of
/// the ctx) — they observe a drained registration list and any
/// further `register()` calls leak harmlessly until the last `Arc`
/// drops.
pub struct ReverseAttachGuard {
    handle: Arc<ReverseAttachHandle>,
}

impl ReverseAttachGuard {
    pub fn new(
        registry: ReverseChannelRegistry,
        sink: SharedSink,
        pending: PendingRequests,
    ) -> Self {
        let handle = Arc::new(ReverseAttachHandle {
            registry,
            channel: ReverseChannel { sink, pending },
            registered: std::sync::Mutex::new(Vec::new()),
        });
        Self { handle }
    }

    /// Returns the shared handle the agent client should stamp on
    /// the per-request `Context` so it can register ids from inside
    /// the swarm-iteration site.
    pub fn handle(&self) -> Arc<ReverseAttachHandle> {
        self.handle.clone()
    }
}

impl Drop for ReverseAttachGuard {
    fn drop(&mut self) {
        let ids = std::mem::take(&mut *self.handle.registered.lock().unwrap());
        for id in ids {
            self.handle.registry.remove(&id);
        }
    }
}
