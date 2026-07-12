//! Reverse-attach handle types.
//!
//! Each CLI WS upgrade owns one [`ReverseChannel`] — the sink to write
//! `server_request` frames out, plus the [`PendingRequests`] slot the
//! recv loop fulfills matching `server_response` frames into. The
//! agent client + retrieval reach it through the [`ReverseAttachHandle`]
//! stamped on the per-request `Context`, and forward requests with
//! [`super::send::send_server_request`].

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

/// Reverse-attach channel for one CLI WS upgrade: the sink to write
/// `server_request` frames out, and the registry to park awaits for
/// matching `server_response` frames coming back.
#[derive(Clone)]
pub struct ReverseChannel {
    pub sink: SharedSink,
    pub pending: PendingRequests,
}

/// Arc-shareable handle the agent client + retrieval use to reach the
/// current WS [`ReverseChannel`] from inside the swarm-iteration site.
/// `Arc` clones may outlive the owning [`ReverseAttachGuard`] (e.g. a
/// background task holding a copy of the ctx) — sends over a closed WS
/// just fail harmlessly.
///
/// There is deliberately NO round-trip budget here: the API waits
/// forever on the CLI client for its own requests. A parked await
/// resolves when the reply arrives or errors when its oneshot sender
/// drops at WS teardown.
pub struct ReverseAttachHandle {
    channel: ReverseChannel,
}

impl std::fmt::Debug for ReverseAttachHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReverseAttachHandle").finish_non_exhaustive()
    }
}

impl ReverseAttachHandle {
    /// The WS reverse channel this upgrade registered against. Callers
    /// that need to send `server_request` frames (e.g. message-queue
    /// reads) reach the sink + pending registry through here.
    pub fn channel(&self) -> &ReverseChannel {
        &self.channel
    }
}

/// RAII guard for one CLI WS upgrade. Owns the shared handle; the
/// agent client stamps a clone on the per-request `Context`.
pub struct ReverseAttachGuard {
    handle: Arc<ReverseAttachHandle>,
}

impl ReverseAttachGuard {
    pub fn new(sink: SharedSink, pending: PendingRequests) -> Self {
        let handle = Arc::new(ReverseAttachHandle {
            channel: ReverseChannel { sink, pending },
        });
        Self { handle }
    }

    /// Returns the shared handle the agent client should stamp on the
    /// per-request `Context`.
    pub fn handle(&self) -> Arc<ReverseAttachHandle> {
        self.handle.clone()
    }
}
