//! The containers a provider is running, by id.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, broadcast, watch};

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
/// derived from anything a caller chose. The map is enumerated one
/// way only, by [`running_under`](Self::running_under), and watched
/// one way only, by [`subscribe`](Self::subscribe): the tool
/// containers one runner holds, and those it starts and ends after,
/// for a
/// [`list_for`](crate::provider::endpoints::containers::tools::list_for)
/// that asks that runner, container by container, before it names
/// any.
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
    /// Every tool container starting and ending, for the listings
    /// open now.
    changes: broadcast::Sender<Change>,
}

/// How many changes a listing may fall behind by before it reads
/// the directory again instead of the changes it missed.
const CHANGES_BEHIND: usize = 256;

/// A tool container came or went: what a listing watches. An agent
/// container, listed to nobody, is neither.
#[derive(Debug, Clone)]
pub enum Change {
    /// A tool container's run has its id: it is running, for its
    /// runner, from now.
    Started(Running),
    /// The run under the id is over.
    Ended {
        /// The container's id.
        id: String,
    },
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

/// One tool container an identity runs, as a listing finds it:
/// enough to ask its runner, and to name it if the runner allows.
#[derive(Debug, Clone)]
pub struct Running {
    /// The container's id.
    pub id: String,
    /// The identity running it.
    pub runner: Arc<str>,
    /// The run scope: where the runner is asked.
    pub scope: Arc<ScopeHandle>,
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
        let (changes, _) = broadcast::channel(CHANGES_BEHIND);
        Directory {
            entries: Mutex::new(HashMap::new()),
            changes,
        }
    }

    /// Every tool container starting and ending from now on, for a
    /// listing: subscribed before the directory is read, so that
    /// nothing between the reading and the watching is missed. A
    /// subscriber that falls behind by more than the feed keeps is
    /// told so, and reads the directory again.
    pub fn subscribe(&self) -> broadcast::Receiver<Change> {
        self.changes.subscribe()
    }

    /// A fresh id.
    pub fn mint() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// The container is running, for `runner`: from now until
    /// [`remove`](Self::remove), connectors may find it, and a
    /// listing hears of a tool container the moment it is in.
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
        let (ended, _) = watch::channel(false);
        let mut entries = self.entries.lock().await;
        let listed = begin.is_some();
        entries.insert(
            id.clone(),
            Entry {
                scope: Arc::clone(&scope),
                runner: Arc::clone(&runner),
                connectors: HashMap::new(),
                proxy,
                begin,
                ignore,
                watched,
                ended,
            },
        );
        if listed {
            // Under the lock, so a listing that read the map just
            // before sees this as a change and one that reads it just
            // after sees it in the map, and neither sees it twice or
            // never. A send with no listing open is nothing.
            let _ = self.changes.send(Change::Started(Running { id, runner, scope }));
        }
    }

    /// The run is over: nothing finds it any more, every connector
    /// attached hears so, and every listing hears a tool container
    /// go.
    pub async fn remove(&self, id: &str) {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.remove(id) {
            // `send_replace`, not `send`: a value sent with no receiver
            // is discarded, and a connector that looks later must still
            // read the end.
            entry.ended.send_replace(true);
            if entry.begin.is_some() {
                let _ = self.changes.send(Change::Ended { id: id.to_string() });
            }
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

    /// Every tool container `runner` is running, in no order: what a
    /// listing asks each runner about, and nothing here is told to
    /// the lister until the runner says so. An agent container is not
    /// among them: it takes no connector, and is listed to nobody.
    pub async fn running_under(&self, runner: &str) -> Vec<Running> {
        self.entries
            .lock()
            .await
            .iter()
            .filter(|(_, entry)| &*entry.runner == runner && entry.begin.is_some())
            .map(|(id, entry)| Running {
                id: id.clone(),
                runner: Arc::clone(&entry.runner),
                scope: Arc::clone(&entry.scope),
            })
            .collect()
    }

    /// The container under `id`, if it is running.
    pub async fn lookup(&self, id: &str) -> Option<Attached> {
        self.entries.lock().await.get(id).map(|entry| Attached {
            scope: Arc::clone(&entry.scope),
            proxy: entry.proxy.clone(),
            begin: entry.begin.clone(),
            ignore: entry.ignore.clone(),
            watched: Arc::clone(&entry.watched),
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
