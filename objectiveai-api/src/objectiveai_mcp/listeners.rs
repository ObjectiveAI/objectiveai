//! Per-`(ws_session_id, mcp_session_id)` broadcast registry feeding
//! the API's `/objectiveai-mcp` MCP GET-SSE stream.
//!
//! - [`McpListenerRegistry::subscribe`] runs from the GET-SSE handler
//!   when a downstream MCP proxy opens its notification stream.
//! - [`McpListenerRegistry::publish`] runs from the conduit's recv
//!   loop whenever a CLI-pushed `McpListChanged` arrives.
//! - [`McpListenerRegistry::gc`] is best-effort cleanup — the
//!   GET-SSE stream's drop guard calls it after the last subscriber
//!   hangs up so we don't leak empty `broadcast::Sender`s.

use dashmap::DashMap;
use objectiveai_sdk::client_objectiveai_mcp::client_request::McpListChangedKind;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Tunable channel depth — large enough that a brief slow consumer
/// doesn't drop list_changed events, small enough that a wedged
/// stream doesn't pin memory. List_changed notifications are
/// inherently bursty (one per upstream change) so even 8 covers a
/// realistic worst case.
const CHANNEL_CAPACITY: usize = 8;

/// Maps `(ws_session_id, mcp_session_id)` to a `broadcast::Sender`
/// the conduit publishes to and the MCP endpoint's GET-SSE handler
/// subscribes from.
///
/// Cheap to clone (`Arc<DashMap<...>>` internally). One instance
/// lives in the API's shared state and is handed to both the recv
/// loop's list-changed dispatcher and the `/objectiveai-mcp` route's
/// GET branch.
#[derive(Clone, Default)]
pub struct McpListenerRegistry {
    inner: Arc<DashMap<(String, String), broadcast::Sender<McpListChangedKind>>>,
}

impl McpListenerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe to the broadcast keyed by `(ws_session_id,
    /// mcp_session_id)`. Creates the channel if no other subscriber
    /// has registered for the same key yet; otherwise joins the
    /// existing one.
    pub fn subscribe(
        &self,
        ws_session_id: &str,
        mcp_session_id: &str,
    ) -> broadcast::Receiver<McpListChangedKind> {
        let key = (ws_session_id.to_string(), mcp_session_id.to_string());
        let entry = self
            .inner
            .entry(key)
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);
        entry.subscribe()
    }

    /// Publish to the broadcast keyed by `(ws_session_id,
    /// mcp_session_id)`. No-op when no subscribers (the expected
    /// state when no proxy is actively listening) — the underlying
    /// `broadcast::send` returns `Err(SendError)` which we
    /// intentionally drop.
    pub fn publish(
        &self,
        ws_session_id: &str,
        mcp_session_id: &str,
        kind: McpListChangedKind,
    ) {
        let key = (ws_session_id.to_string(), mcp_session_id.to_string());
        if let Some(tx) = self.inner.get(&key) {
            let _ = tx.send(kind);
        }
    }

    /// Drop the broadcast for `(ws_session_id, mcp_session_id)` if
    /// no receivers remain. Called from the GET-SSE stream's drop
    /// guard so an idle pair doesn't pin a `Sender` forever.
    pub fn gc(&self, ws_session_id: &str, mcp_session_id: &str) {
        let key = (ws_session_id.to_string(), mcp_session_id.to_string());
        // `remove_if` here is racy in the strict sense (a concurrent
        // `subscribe` could race in), but the worst case is a
        // resurrected channel — the next `publish` simply succeeds
        // again. Acceptable.
        self.inner
            .remove_if(&key, |_, tx| tx.receiver_count() == 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subscribe_then_publish_round_trips_kind() {
        let reg = McpListenerRegistry::new();
        let mut rx = reg.subscribe("ws", "mcp");
        reg.publish("ws", "mcp", McpListChangedKind::Tools);
        let got = rx.recv().await.expect("recv");
        assert_eq!(got, McpListChangedKind::Tools);
    }

    #[tokio::test]
    async fn publish_with_no_subscribers_is_silent_no_op() {
        let reg = McpListenerRegistry::new();
        // No panic, no error.
        reg.publish("ws", "mcp", McpListChangedKind::Resources);
    }

    #[tokio::test]
    async fn gc_drops_channel_only_when_no_receivers() {
        let reg = McpListenerRegistry::new();
        let rx = reg.subscribe("ws", "mcp");
        // With one receiver, gc is a no-op.
        reg.gc("ws", "mcp");
        // Drop the receiver, then gc should remove.
        drop(rx);
        reg.gc("ws", "mcp");
        // A fresh subscribe should land on a brand-new channel.
        let mut rx2 = reg.subscribe("ws", "mcp");
        reg.publish("ws", "mcp", McpListChangedKind::Tools);
        assert_eq!(rx2.recv().await.unwrap(), McpListChangedKind::Tools);
    }
}
