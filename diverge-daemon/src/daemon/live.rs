//! What is live: nobody's record, gone at restart.

use std::collections::{HashMap, HashSet};

use tokio::sync::Mutex;

use crate::store::AccountId;

/// The live state of the daemon. Today, which accounts have a client
/// connected as them, and how many; later, which containers run under
/// which account. Behind an async mutex, held for a lookup and never
/// across anything that waits on the outside.
#[derive(Debug, Default)]
pub struct Live {
    /// Connections per account, for the accounts with any.
    connected: Mutex<HashMap<AccountId, usize>>,
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
}
