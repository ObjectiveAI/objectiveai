//! The containers a provider is running, by id.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, watch};

use crate::wire::server::scope_handle::ScopeHandle;
use crate::provider::endpoints::containers::server::watched::Watched;
use crate::wire::client::handle::Handle;
use crate::container_proxy::outside::endpoints::tools::begin::client::execute::ExecuteHandle as ToolsBegin;

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
/// container once the id is minted, announces it once the id is sent,
/// and removes it when the run ends; a connect handler looks its id
/// up — getting the run scope, to ask the runner whether the
/// connector may attach and to tell the runner it did; the connection
/// to the container's proxy and the begin scope on it, which the
/// connector's channels ride; a signal for the id having been sent,
/// before which no connector is attached; and a signal for the run
/// ending, which ends every connection to it.
///
/// # The id is a capability
///
/// Minted here, by [`mint`](Self::mint), as a v4 UUID: holding one is
/// what lets a connector ask, so it is unguessable and never
/// derived from anything a caller chose. The map is never enumerated
/// and never watched: a container is found by its id, and by nothing
/// else.
///
/// # And who may reach each container
///
/// The identity that runs each container, and every identity
/// connected to it as a connector, are kept beside it — set by the
/// run handler and by the connect handler as a connector attaches and
/// leaves — so that a transfer, which names a container by id, can be
/// held to its rule: the caller must be running or connected to the
/// container it names; and so that a
/// [`serve`](crate::provider::endpoints::containers::serve), which names
/// one the same way, can be held to its stricter one: the caller must
/// be running it. Nothing else reads them, and an id is still never
/// enumerated.
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
    /// `true` once the id has been sent on the run scope: nothing is
    /// sent on that scope's main stream for a connector before it.
    announced: watch::Sender<bool>,
    ended: watch::Sender<bool>,
}

/// What a connector is handed for a container it named.
#[derive(Debug, Clone)]
pub struct Attached {
    /// The run scope: where the runner is asked, and told.
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
    /// `true` once the run's id has been sent, which a connector
    /// waits for before it is attached. A receiver whose sender is
    /// gone reads as the run ended.
    pub announced: watch::Receiver<bool>,
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
    /// [`remove`](Self::remove), connectors may find it — and are
    /// attached only once it is [`announce`](Self::announce)d.
    pub(crate) async fn insert(
        &self,
        id: String,
        scope: Arc<ScopeHandle>,
        runner: Arc<str>,
        proxy: Handle,
        begin: Option<ToolsBegin>,
        ignore: Vec<Vec<String>>,
        watched: Arc<[Watched]>,
    ) {
        let (announced, _) = watch::channel(false);
        let (ended, _) = watch::channel(false);
        let mut entries = self.entries.lock().await;
        entries.insert(
            id,
            Entry {
                scope,
                runner,
                connectors: HashMap::new(),
                proxy,
                begin,
                ignore,
                watched,
                announced,
                ended,
            },
        );
    }

    /// The run's id has been sent: a connector waiting on it is
    /// attached from now, and the first response on the run scope is
    /// behind every one a connector causes.
    pub(crate) async fn announce(&self, id: &str) {
        if let Some(entry) = self.entries.lock().await.get(id) {
            entry.announced.send_replace(true);
        }
    }

    /// The run is over: nothing finds it any more, and every connector
    /// attached hears so.
    pub async fn remove(&self, id: &str) {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.remove(id) {
            // `send_replace`, not `send`: a value sent with no receiver
            // is discarded, and a connector that looks later must still
            // read the end.
            entry.ended.send_replace(true);
        }
    }

    /// `identity` is attached to the container under `id` as a
    /// connector, from now until [`detach`](Self::detach). `false` is
    /// an id under which nothing is running, and nothing changed.
    pub async fn attach(&self, id: &str, identity: &Arc<str>) -> bool {
        let mut entries = self.entries.lock().await;
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
    pub async fn detach(&self, id: &str, identity: &str) {
        let mut entries = self.entries.lock().await;
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
    pub async fn may(&self, id: &str, identity: &str) -> bool {
        self.entries
            .lock()
            .await
            .get(id)
            .is_some_and(|entry| &*entry.runner == identity || entry.connectors.contains_key(identity))
    }

    /// Whether `identity` is running the container under `id`: the
    /// runner alone, no connector. `false` for an id under which
    /// nothing is running, indistinguishably, as [`may`](Self::may)
    /// is. What a serve of the container asks, since a serve is the
    /// runner's alone.
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
            scope: Arc::clone(&entry.scope),
            proxy: entry.proxy.clone(),
            begin: entry.begin.clone(),
            ignore: entry.ignore.clone(),
            watched: Arc::clone(&entry.watched),
            announced: entry.announced.subscribe(),
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
