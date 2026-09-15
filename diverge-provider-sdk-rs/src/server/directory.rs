//! The containers a provider is running, by id.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::watch;

use super::scope_handle::ScopeHandle;
use crate::client::handle::Handle;
use crate::container_proxy_endpoints::tools::begin::client::execute::ExecuteHandle as ToolsBegin;

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
/// ask the runner whether the connector may attach; the connection to
/// the container's proxy and the begin scope on it, which the
/// connector's channels ride; and a signal for the run ending, which
/// ends every connection to it.
///
/// # The id is a capability
///
/// Minted here, by [`mint`](Self::mint), as a v4 UUID: holding one is
/// what lets a connector ask, so it is unguessable and never
/// derived from anything a caller chose. Nothing enumerates the map.
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
    scope: Arc<ScopeHandle>,
    proxy: Handle,
    begin: Option<ToolsBegin>,
    ignore: Vec<Vec<String>>,
    ended: watch::Sender<bool>,
}

/// What a connector is handed for a container it named.
#[derive(Debug, Clone)]
pub struct Attached {
    /// The run scope: where the runner is asked.
    pub scope: Arc<ScopeHandle>,
    /// The one connection to the container's proxy, which the
    /// connector's tree, read and write scopes are opened on.
    pub proxy: Handle,
    /// The run's begin scope, on which a connector's MCP exchanges
    /// are channels — [`None`] for an agent container, which is its
    /// runner's alone and takes no connector.
    pub begin: Option<ToolsBegin>,
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
        }
    }

    /// A fresh id.
    pub fn mint() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// The container is running: from now until
    /// [`remove`](Self::remove), connectors may find it.
    pub fn insert(
        &self,
        id: String,
        scope: Arc<ScopeHandle>,
        proxy: Handle,
        begin: Option<ToolsBegin>,
        ignore: Vec<Vec<String>>,
    ) {
        let (ended, _) = watch::channel(false);
        self.lock().insert(
            id,
            Entry {
                scope,
                proxy,
                begin,
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
            proxy: entry.proxy.clone(),
            begin: entry.begin.clone(),
            ignore: entry.ignore.clone(),
            ended: entry.ended.subscribe(),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        // Nothing awaits under the lock and nothing panics under it;
        // a poisoned map would be one whose contents are still right.
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
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
