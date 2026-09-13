//! The gate the notifications ask waits behind.

use tokio::sync::watch;

/// Whether the agent has made its first MCP exchange.
///
/// Opened by the first list-tools, list-resources, call-tool or
/// read-resource the agent asks — those four and nothing else: not
/// a session initializing, and not whatever else may ride
/// `/requests` in time. Until it opens, the proxy sends no
/// notifications ask, so a container whose agent never touches MCP
/// costs the caller nothing. Once open it stays open: the
/// notifications stream is then kept for the proxy's life.
pub struct Gate(watch::Sender<bool>);

impl Gate {
    pub fn new() -> Self {
        Self(watch::Sender::new(false))
    }

    /// Open it. Idempotent — the second exchange changes nothing.
    pub fn open(&self) {
        self.0.send_replace(true);
    }

    /// Wait until it is open; at once if it already is.
    pub async fn opened(&self) {
        let mut watcher = self.0.subscribe();
        let _ = watcher.wait_for(|opened| *opened).await;
    }
}
