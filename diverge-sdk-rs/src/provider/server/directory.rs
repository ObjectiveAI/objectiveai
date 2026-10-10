//! The containers a provider is running, by id.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, watch};

use crate::wire::client::handle::Handle;

/// Every container running under this provider, by the
/// [`Id`](crate::shared::containers::response::Id) its run was
/// given — what a [`serve`](crate::provider::endpoints::containers::serve)
/// and a transfer have to find, on whatever connection they arrive.
///
/// One per provider, shared across every connection's
/// [`handle`](super::handle::handle): a serve names a container its
/// runner may have started from another socket entirely, so the map
/// cannot live in a connection. A run handler inserts its container
/// once the id is minted and removes it when the run ends; a serve
/// handler and a transfer look an id up — getting the connection to
/// the container's proxy, which their scopes ride, and a signal for
/// the run ending.
///
/// # The id is a capability
///
/// Minted here, by [`mint`](Self::mint), as a v4 UUID: holding one is
/// what lets its runner name the container elsewhere, so it is
/// unguessable and never derived from anything a caller chose. The
/// map is never enumerated and never watched: a container is found by
/// its id, and by nothing else.
///
/// # And who may reach each container
///
/// The identity that runs each container is kept beside it, so that a
/// serve and a transfer, which name a container by id, can be held to
/// their rule: the caller must be running it. Nothing else reads it,
/// and an id is still never enumerated.
///
/// # Not the volumes in use
///
/// Which volumes are mounted in a running container is not kept
/// here: it is the [`lock`](super::volume::Volume::lock) on each
/// volume, which the provider keeps and the run handler takes and
/// gives back. This is containers only.
pub struct Directory {
    entries: Mutex<HashMap<String, Entry>>,
}

/// One running container.
struct Entry {
    /// The identity running the container.
    runner: Arc<str>,
    proxy: Handle,
    ended: watch::Sender<bool>,
}

/// What a serve or a transfer is handed for a container it named.
#[derive(Debug, Clone)]
pub struct Attached {
    /// The one connection to the container's proxy, which a serve's
    /// asks and a transfer's write are opened on.
    pub proxy: Handle,
    /// `true` once the run is over. A receiver whose sender is gone
    /// reads the same.
    pub ended: watch::Receiver<bool>,
}

impl Directory {
    pub fn new() -> Self {
        Directory {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// A fresh id.
    pub fn mint() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// The container is running, for `runner`: from now until
    /// [`remove`](Self::remove), its runner's serves and transfers
    /// find it.
    pub(crate) async fn insert(&self, id: String, runner: Arc<str>, proxy: Handle) {
        let (ended, _) = watch::channel(false);
        let mut entries = self.entries.lock().await;
        entries.insert(id, Entry { runner, proxy, ended });
    }

    /// The run is over: nothing finds it any more, and every serve on
    /// it hears so.
    pub async fn remove(&self, id: &str) {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.remove(id) {
            // `send_replace`, not `send`: a value sent with no receiver
            // is discarded, and a serve that looks later must still
            // read the end.
            entry.ended.send_replace(true);
        }
    }

    /// Whether `identity` is running the container under `id`. `false`
    /// for an id under which nothing is running, indistinguishably:
    /// whether an id exists is not told to a caller that may not reach
    /// it. What a serve of the container asks, and what a transfer into
    /// it asks.
    pub async fn runs(&self, id: &str, identity: &str) -> bool {
        self.entries
            .lock()
            .await
            .get(id)
            .is_some_and(|entry| &*entry.runner == identity)
    }

    /// The container under `id`, if it is running.
    pub async fn lookup(&self, id: &str) -> Option<Attached> {
        self.entries.lock().await.get(id).map(|entry| Attached {
            proxy: entry.proxy.clone(),
            ended: entry.ended.subscribe(),
        })
    }
}

impl Default for Directory {
    fn default() -> Self {
        Directory::new()
    }
}

impl std::fmt::Debug for Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Directory");
        match self.entries.try_lock() {
            Ok(entries) => debug.field("running", &entries.len()),
            Err(_) => debug.field("running", &"locked"),
        };
        debug.finish()
    }
}
