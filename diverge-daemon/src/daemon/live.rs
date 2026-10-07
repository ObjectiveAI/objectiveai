//! What is live: nobody's record, gone at restart.

use std::collections::{HashMap, HashSet};

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::client::handle::Handle;
use tokio::sync::Mutex;
use tokio::task::AbortHandle;

use crate::store::AccountId;

/// The live state of the daemon: which accounts have a client
/// connected as them, and how many; which providers the daemon holds
/// a connection to, and the handle it speaks to each on; and the dial
/// tasks keeping the outgoing ones connected. Later, which containers
/// run under which account. Behind async mutexes, held for a lookup
/// and never across anything that waits on the outside.
#[derive(Debug, Default)]
pub struct Live {
    /// Connections per account, for the accounts with any.
    connected: Mutex<HashMap<AccountId, usize>>,
    /// The providers connected now, by identity, and the caller's
    /// handle on each: what every request to a provider rides.
    providers: Mutex<HashMap<Identity, Handle>>,
    /// The dial task of every outgoing provider, by address, to be
    /// ended when the provider is forgotten or the daemon stops.
    dials: Mutex<HashMap<String, AbortHandle>>,
}

impl Live {
    /// Nothing live.
    pub fn new() -> Self {
        Live::default()
    }

    /// Count a connection as the account in. The connection counts
    /// itself out with [`leave`](Self::leave) when it ends.
    pub async fn enter(&self, id: AccountId) {
        *self.connected.lock().await.entry(id).or_insert(0) += 1;
    }

    /// Count a connection as the account out.
    pub async fn leave(&self, id: AccountId) {
        let mut connected = self.connected.lock().await;
        if let Some(count) = connected.get_mut(&id) {
            *count -= 1;
            if *count == 0 {
                connected.remove(&id);
            }
        }
    }

    /// Whether a client is connected as the account now.
    pub async fn is_connected(&self, id: AccountId) -> bool {
        self.connected.lock().await.contains_key(&id)
    }

    /// Every account a client is connected as now: one snapshot, for
    /// a list that asks about each.
    pub async fn connected(&self) -> HashSet<AccountId> {
        self.connected.lock().await.keys().copied().collect()
    }

    /// Whether anything holds the account — a client connected as it,
    /// or a container running under it — which is what refuses its
    /// deletion. No container runs under anything yet.
    pub async fn holds(&self, id: AccountId) -> bool {
        self.is_connected(id).await
    }

    /// A provider is connected, and this is the handle on it. A
    /// second connection under one identity replaces the first's
    /// handle, which is the newer one being the one that lives.
    pub async fn connect_provider(&self, identity: Identity, handle: Handle) {
        self.providers.lock().await.insert(identity, handle);
    }

    /// The provider's connection ended.
    pub async fn disconnect_provider(&self, identity: &Identity) {
        self.providers.lock().await.remove(identity);
    }

    /// Whether the daemon holds a connection to the provider now.
    pub async fn is_provider_connected(&self, identity: &Identity) -> bool {
        self.providers.lock().await.contains_key(identity)
    }

    /// Every provider connected now: one snapshot, for a list.
    pub async fn connected_providers(&self) -> HashSet<Identity> {
        self.providers.lock().await.keys().cloned().collect()
    }

    /// The handle on the provider, if it is connected.
    pub async fn provider(&self, identity: &Identity) -> Option<Handle> {
        self.providers.lock().await.get(identity).cloned()
    }

    /// Keep the dial task for the address, ending any earlier one for
    /// the same address first, so one address is dialled by one task.
    pub async fn start_dial(&self, address: String, dial: AbortHandle) {
        if let Some(earlier) = self.dials.lock().await.insert(address, dial) {
            earlier.abort();
        }
    }

    /// End the dial task for the address, and the connection it holds
    /// with it — and take the handle out, since a task ended mid-way
    /// does not get to. An address with no dial is nothing to end.
    pub async fn stop_dial(&self, address: &str) {
        if let Some(dial) = self.dials.lock().await.remove(address) {
            dial.abort();
        }
        self.disconnect_provider(&Identity::Outgoing {
            address: address.to_string(),
        })
        .await;
    }

    /// End every dial task: the daemon is stopping, and nothing reads
    /// the registry after.
    pub async fn stop_dials(&self) {
        for (_, dial) in self.dials.lock().await.drain() {
            dial.abort();
        }
    }
}
