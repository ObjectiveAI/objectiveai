//! Every MCP session the proxy is serving, as broadcast targets.

use std::collections::HashMap;

use rmcp::RoleServer;
use rmcp::model::ServerNotification;
use rmcp::service::Peer;
use tokio::sync::Mutex;

/// The live sessions' peers, keyed by an id minted at registration.
///
/// A session enters when it initializes and leaves only by failing a
/// send — the proxy never closes a session's stream, it merely stops
/// addressing one that is provably gone. There is nothing else here:
/// no per-session state, no queue, no replay. A notification is
/// shipped to whoever is registered at the moment it arrives, once.
pub struct Peers {
    inner: Mutex<Registry>,
}

struct Registry {
    peers: HashMap<u64, Peer<RoleServer>>,
    /// The next id. Counted here, under the same lock that uses it.
    next: u64,
}

impl Peers {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Registry {
                peers: HashMap::new(),
                next: 0,
            }),
        }
    }

    /// Register a session's peer. From here on it receives every
    /// broadcast until a send to it fails.
    pub async fn insert(&self, peer: Peer<RoleServer>) {
        let mut registry = self.inner.lock().await;
        let id = registry.next;
        registry.next += 1;
        registry.peers.insert(id, peer);
    }

    /// Ship one notification to every registered peer, once.
    ///
    /// The sends run concurrently ACROSS peers and are awaited, so a
    /// single notification is one round of fan-out and per-peer order
    /// is the order of the broadcasts — the resident stream calls this
    /// one notification at a time. The trade is stated where it is
    /// made: a stalled session stalls the round, which in a container
    /// serving one agent is a corner accepted rather than a queue
    /// grown.
    ///
    /// A peer whose send failed is removed: its session is gone, and
    /// there is nobody behind it to address. Removal is by id, so a
    /// registration that happened during the round is untouched.
    pub async fn broadcast(&self, notification: ServerNotification) {
        let snapshot: Vec<(u64, Peer<RoleServer>)> = {
            let registry = self.inner.lock().await;
            registry
                .peers
                .iter()
                .map(|(id, peer)| (*id, peer.clone()))
                .collect()
        };
        if snapshot.is_empty() {
            return;
        }

        let sends = snapshot.into_iter().map(|(id, peer)| {
            let notification = notification.clone();
            async move {
                match peer.send_notification(notification).await {
                    Ok(()) => None,
                    Err(_) => Some(id),
                }
            }
        });
        let failed: Vec<u64> = futures_util::future::join_all(sends)
            .await
            .into_iter()
            .flatten()
            .collect();

        if !failed.is_empty() {
            let mut registry = self.inner.lock().await;
            for id in failed {
                registry.peers.remove(&id);
            }
        }
    }
}
