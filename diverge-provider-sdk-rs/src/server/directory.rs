//! The containers a provider is running, by id.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use tokio::sync::watch;

use super::scope_handle::ScopeHandle;

/// Every container running under this provider, by the
/// [`Id`](crate::shared::containers::response::Id) its run was
/// given — what a
/// [`connect`](crate::endpoints::containers::tools::connect) has to
/// find, on whatever connection it arrives.
///
/// One per provider, shared across every connection's
/// [`handle`](super::handle::handle): a connector names a container
/// its runner may have started from another socket entirely, so the
/// map cannot live in a connection. A run handler inserts its
/// container once the id is minted and removes it when the run ends,
/// and a connect handler looks its id up — getting the run scope, to
/// ask the runner whether the connector may attach; the address, to
/// dial the container; and a signal for the run ending, which ends
/// every connection to it.
///
/// # The id is a capability
///
/// Minted here, by [`mint`](Self::mint), as a v4 UUID: holding one is
/// what lets a connector ask, so it is unguessable and never
/// derived from anything a caller chose. Nothing enumerates the map.
///
/// # And the volumes in use
///
/// A volume is mounted in at most one container of its caller at a
/// time, and this is where that is kept: a run handler
/// [`lease`](Self::lease)s every volume its request names before it
/// fetches or deploys anything, and [`release`](Self::release)s them
/// when the run ends, on every path. What the specification calls
/// "mounted in a running container" is a lease held here — a
/// [`delete`](crate::endpoints::volumes::delete) asks
/// [`mounted`](Self::mounted) before it asks the manager.
pub struct Directory {
    entries: Mutex<HashMap<String, Entry>>,
    /// `(client identity, volume name)` for every volume mounted in a
    /// running container.
    volumes: Mutex<HashSet<(String, String)>>,
}

/// One running container.
struct Entry {
    scope: Arc<ScopeHandle>,
    address: String,
    ignore: Vec<Vec<String>>,
    ended: watch::Sender<bool>,
}

/// What a connector is handed for a container it named.
#[derive(Debug, Clone)]
pub struct Attached {
    /// The run scope: where the runner is asked.
    pub scope: Arc<ScopeHandle>,
    /// The container's proxy, as the run's deployer reported it.
    pub address: String,
    /// Every mount's path, which a filetree the connector opens
    /// leaves out as the runner's does.
    pub ignore: Vec<Vec<String>>,
    /// `true` once the run is over. A receiver whose sender is gone
    /// reads the same.
    pub ended: watch::Receiver<bool>,
}

impl Directory {
    pub fn new() -> Self {
        Directory {
            entries: Mutex::new(HashMap::new()),
            volumes: Mutex::new(HashSet::new()),
        }
    }

    /// Hold every volume in `names` for `identity`, or none of them.
    ///
    /// Atomic: either every name was free and all are now held, or
    /// one was held already — by another running container of this
    /// identity, or twice in this list — and nothing changed, with
    /// that name the error. The run that leased them
    /// [`release`](Self::release)s them when it ends.
    pub fn lease(&self, identity: &str, names: &[String]) -> Result<(), String> {
        let mut held = self.lock_volumes();
        let mut taken: Vec<(String, String)> = Vec::with_capacity(names.len());
        for name in names {
            let key = (identity.to_string(), name.clone());
            if held.contains(&key) || taken.contains(&key) {
                return Err(name.clone());
            }
            taken.push(key);
        }
        held.extend(taken);
        Ok(())
    }

    /// The run is over: its volumes are free again.
    pub fn release(&self, identity: &str, names: &[String]) {
        let mut held = self.lock_volumes();
        for name in names {
            held.remove(&(identity.to_string(), name.clone()));
        }
    }

    /// Whether `name` is mounted in a running container of
    /// `identity`.
    pub fn mounted(&self, identity: &str, name: &str) -> bool {
        self.lock_volumes()
            .contains(&(identity.to_string(), name.to_string()))
    }

    /// A fresh id.
    pub fn mint() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// The container is running: from now until
    /// [`remove`](Self::remove), connectors may find it.
    pub fn insert(&self, id: String, scope: Arc<ScopeHandle>, address: String, ignore: Vec<Vec<String>>) {
        let (ended, _) = watch::channel(false);
        self.lock().insert(
            id,
            Entry {
                scope,
                address,
                ignore,
                ended,
            },
        );
    }

    /// The run is over: nothing finds it any more, and every
    /// connector attached hears so.
    pub fn remove(&self, id: &str) {
        if let Some(entry) = self.lock().remove(id) {
            // `send_replace`, not `send`: a value sent with no receiver
            // is discarded, and a connector that looks later must still
            // read the end.
            entry.ended.send_replace(true);
        }
    }

    /// The container under `id`, if it is running.
    pub fn lookup(&self, id: &str) -> Option<Attached> {
        self.lock().get(id).map(|entry| Attached {
            scope: Arc::clone(&entry.scope),
            address: entry.address.clone(),
            ignore: entry.ignore.clone(),
            ended: entry.ended.subscribe(),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        // Nothing awaits under the lock and nothing panics under it;
        // a poisoned map would be one whose contents are still right.
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_volumes(&self) -> std::sync::MutexGuard<'_, HashSet<(String, String)>> {
        self.volumes.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for Directory {
    fn default() -> Self {
        Directory::new()
    }
}

impl std::fmt::Debug for Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Directory").field("running", &self.lock().len()).finish()
    }
}
