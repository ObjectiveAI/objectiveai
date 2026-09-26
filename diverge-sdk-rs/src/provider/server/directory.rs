//! The containers a provider is running, by id.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::watch;

use crate::wire::server::scope_handle::ScopeHandle;
use crate::provider::endpoints::containers::server::watched::Watched;
use crate::wire::client::handle::Handle;
use crate::container_proxy::outside::tools::begin::client::execute::ExecuteHandle as ToolsBegin;

/// Every container running under this provider, by the
/// [`Id`](crate::shared::containers::response::Id) its run was
/// given — what a
/// [`connect`](crate::provider::endpoints::containers::tools::connect) has to
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
/// # And who may reach each container
///
/// The identity that runs each container, and every identity
/// connected to it as a connector, are kept beside it — set by the
/// run handler and by the connect handler as a connector attaches and
/// leaves — so that a transfer, which names a container by id, can be
/// held to its rule: the caller must be running or connected to the
/// container it names. Nothing else reads them, and an id is still
/// never enumerated.
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
    /// The identity running the container.
    runner: Arc<str>,
    /// Every identity attached as a connector, with how many of its
    /// connections are: one identity may connect twice, and leaves
    /// when the last of them does.
    connectors: HashMap<Arc<str>, usize>,
    proxy: Handle,
    begin: Option<ToolsBegin>,
    /// Every path the proxy's tree leaves out.
    ignore: Vec<Vec<String>>,
    /// Every mount the provider watches itself, for a filetree.
    watched: Arc<[Watched]>,
    ended: watch::Sender<bool>,
}

/// What a connector is handed for a container it named.
#[derive(Debug, Clone)]
pub struct Attached {
    /// The run scope: where the runner is asked.
    pub scope: Arc<ScopeHandle>,
    /// The one connection to the container's proxy, which the
    /// connector's tree, read, write and transfer scopes are opened
    /// on.
    pub proxy: Handle,
    /// The run's begin scope, on which a connector's MCP exchanges
    /// are channels — [`None`] for an agent container, which is its
    /// runner's alone and takes no connector.
    pub begin: Option<ToolsBegin>,
    /// Every path the proxy's tree leaves out, for a filetree the
    /// connector opens as for the runner's.
    pub ignore: Vec<Vec<String>>,
    /// Every mount the provider watches itself, which a filetree the
    /// connector opens merges in as the runner's does.
    pub(crate) watched: Arc<[Watched]>,
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
    /// [`remove`](Self::remove), connectors may find it.
    pub(crate) fn insert(
        &self,
        id: String,
        scope: Arc<ScopeHandle>,
        runner: Arc<str>,
        proxy: Handle,
        begin: Option<ToolsBegin>,
        ignore: Vec<Vec<String>>,
        watched: Arc<[Watched]>,
    ) {
        let (ended, _) = watch::channel(false);
        self.lock().insert(
            id,
            Entry {
                scope,
                runner,
                connectors: HashMap::new(),
                proxy,
                begin,
                ignore,
                watched,
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

    /// `identity` is attached to the container under `id` as a
    /// connector, from now until [`detach`](Self::detach). `false` is
    /// an id under which nothing is running, and nothing changed.
    pub fn attach(&self, id: &str, identity: &Arc<str>) -> bool {
        let mut entries = self.lock();
        let Some(entry) = entries.get_mut(id) else {
            return false;
        };
        *entry.connectors.entry(Arc::clone(identity)).or_insert(0) += 1;
        true
    }

    /// One of `identity`'s connections to the container under `id`
    /// has left; the identity is a connector until the last of them
    /// does. An id no longer running, or an identity not attached,
    /// is nothing to do.
    pub fn detach(&self, id: &str, identity: &str) {
        let mut entries = self.lock();
        let Some(entry) = entries.get_mut(id) else {
            return;
        };
        let Some(count) = entry.connectors.get_mut(identity) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            entry.connectors.remove(identity);
        }
    }

    /// Whether `identity` is running the container under `id`, or is
    /// attached to it as a connector. `false` for an id under which
    /// nothing is running, indistinguishably: whether an id exists is
    /// not told to a caller that may not reach it.
    pub fn may(&self, id: &str, identity: &str) -> bool {
        self.lock()
            .get(id)
            .is_some_and(|entry| &*entry.runner == identity || entry.connectors.contains_key(identity))
    }

    /// The container under `id`, if it is running.
    pub fn lookup(&self, id: &str) -> Option<Attached> {
        self.lock().get(id).map(|entry| Attached {
            scope: Arc::clone(&entry.scope),
            proxy: entry.proxy.clone(),
            begin: entry.begin.clone(),
            ignore: entry.ignore.clone(),
            watched: Arc::clone(&entry.watched),
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
