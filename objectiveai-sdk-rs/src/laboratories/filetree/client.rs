//! Materialized consumer of a laboratory's `/filetree` SSE endpoint.
//!
//! [`FileTree`] is NOT a raw event stream — it connects once, then
//! folds every incoming [`FileTreeEvent`] into an in-memory,
//! self-updating map of `path → FileTreeEntry`: a
//! [`Snapshot`](FileTreeEvent::Snapshot) replaces the whole set,
//! [`Upserted`](FileTreeEvent::Upserted) replaces one entry by path
//! (introducing it if unseen), and [`Removed`](FileTreeEvent::Removed)
//! drops one path and its whole subtree.
//!
//! Three ways to observe it:
//! - [`entries`](FileTree::entries) — async snapshot of the current
//!   set (sorted by path).
//! - [`tree`](FileTree::tree) — the current set rebuilt into the
//!   recursive [`FileTreeNode`] shape.
//! - an on-change **callback**
//!   ([`on_change`](FileTreeBuilder::on_change)), invoked with the full
//!   refreshed set on every applied change.
//! - [`subscribe`](FileTree::subscribe) — async, blocks until the next
//!   change.
//!
//! One [`FileTree`] = one connection: the internal pump runs until the
//! laboratory closes the stream; after that the view is frozen at its
//! last state. Dropping it aborts the pump. Reconnection is the
//! caller's loop — build a new [`FileTree`].

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{Mutex, watch};

use super::{FileKind, FileTreeEntry, FileTreeEvent, FileTreeNode};

/// The on-change callback: invoked with the full current entry set
/// (sorted by path) after each applied [`FileTreeEvent`].
pub type OnChange = Box<dyn Fn(&[FileTreeEntry]) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request builder rejected the URL, or opening the SSE
    /// stream failed.
    #[error("connect laboratory filetree: {0}")]
    Connect(#[from] reqwest_eventsource::CannotCloneRequestError),
    /// The underlying reqwest client failed to build.
    #[error("laboratory filetree http client: {0}")]
    Client(#[from] reqwest::Error),
}

/// The shared inner state, held by both the [`FileTree`] handle and
/// its pump task.
struct Shared {
    /// `path → entry`. A `BTreeMap` so iteration (snapshots, the
    /// callback) is sorted by path.
    state: Mutex<BTreeMap<String, FileTreeEntry>>,
    /// A monotonically-bumped change counter. Each applied event bumps
    /// it, waking every [`subscribe`](FileTree::subscribe) waiter.
    changes: watch::Sender<u64>,
    /// Optional push callback, invoked with the full set after each
    /// change.
    on_change: Option<OnChange>,
}

impl Shared {
    fn entries(state: &BTreeMap<String, FileTreeEntry>) -> Vec<FileTreeEntry> {
        state.values().cloned().collect()
    }
}

/// Unconnected configuration — [`FileTree::new`] +
/// [`FileTreeBuilder::on_change`] + [`FileTreeBuilder::connect`].
pub struct FileTreeBuilder {
    /// The laboratory's base HTTP address, e.g.
    /// `http://127.0.0.1:49152`.
    base_url: String,
    /// Optional absolute path inside the container to watch. `None`
    /// watches the whole filesystem (`/`).
    path: Option<String>,
    /// Optional on-change callback.
    on_change: Option<OnChange>,
}

impl FileTreeBuilder {
    /// Scope the watch to an absolute path inside the container. Omit
    /// to watch the whole filesystem.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Register a callback invoked with the full current entry set
    /// (sorted by path) after every applied change. Runs on the pump
    /// task, so keep it cheap and non-blocking; for the full state on
    /// demand use [`entries`](FileTree::entries).
    pub fn on_change(
        mut self,
        callback: impl Fn(&[FileTreeEntry]) + Send + Sync + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Open the SSE stream and start the pump. The returned
    /// [`FileTree`] immediately begins folding events (the first is
    /// the endpoint's connect-time snapshot). Laboratories have no
    /// auth — the endpoint rides the same loopback-published port the
    /// conduit dials.
    pub async fn connect(self) -> Result<FileTree, Error> {
        let client = reqwest::Client::builder().build()?;
        let mut request = client.get(format!("{}/filetree", self.base_url));
        if let Some(path) = &self.path {
            request = request.query(&[("path", path)]);
        }
        let source = request.eventsource()?;

        let shared = Arc::new(Shared {
            state: Mutex::new(BTreeMap::new()),
            changes: watch::channel(0u64).0,
            on_change: self.on_change,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(FileTree { shared, pump })
    }
}

/// The materialized `/filetree` view — see the module docs. Construct
/// via [`FileTree::new`]. Dropping it aborts the background pump.
pub struct FileTree {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl FileTree {
    /// Start building a [`FileTree`] for a laboratory's base HTTP
    /// address (e.g. `http://127.0.0.1:<port>`).
    pub fn new(base_url: impl Into<String>) -> FileTreeBuilder {
        FileTreeBuilder {
            base_url: base_url.into(),
            path: None,
            on_change: None,
        }
    }

    /// Snapshot the current entry set, sorted by path.
    pub async fn entries(&self) -> Vec<FileTreeEntry> {
        Shared::entries(&*self.shared.state.lock().await)
    }

    /// Rebuild the current entry set into the recursive
    /// [`FileTreeNode`] shape. The returned root's `name` is empty
    /// (the watched dir); its children are the tree. `None` before the
    /// first snapshot has been folded.
    pub async fn tree(&self) -> Option<FileTreeNode> {
        let state = self.shared.state.lock().await;
        if state.is_empty() {
            return None;
        }
        Some(rebuild_tree(&state))
    }

    /// Block until the next change is applied. A fresh call waits for
    /// the FIRST change after it is made, so a change that lands
    /// between a preceding [`entries`](Self::entries) read and this
    /// call is not observed by it — pair with the read in a loop, or
    /// use the [`on_change`](FileTreeBuilder::on_change) callback for
    /// guaranteed push.
    pub async fn subscribe(&self) {
        let mut rx = self.shared.changes.subscribe();
        let _ = rx.changed().await;
    }
}

impl Drop for FileTree {
    fn drop(&mut self) {
        self.pump.abort();
    }
}

/// Fold one event into the map. `Snapshot` replaces the whole set;
/// `Upserted` replaces one entry by path; `Removed` drops the path and
/// its whole subtree (any key equal to `path` or under `path/`).
fn apply_event(state: &mut BTreeMap<String, FileTreeEntry>, event: FileTreeEvent) {
    match event {
        FileTreeEvent::Snapshot { root } => {
            state.clear();
            for (path, entry) in root.flatten() {
                state.insert(path, entry);
            }
        }
        FileTreeEvent::Upserted { entry } => {
            state.insert(entry.path.clone(), entry);
        }
        FileTreeEvent::Removed { path } => {
            let subtree_prefix = format!("{path}/");
            state.retain(|key, _| key != &path && !key.starts_with(&subtree_prefix));
        }
    }
}

/// Rebuild the flat `path → entry` map into a recursive
/// [`FileTreeNode`] rooted at an empty-named node.
fn rebuild_tree(state: &BTreeMap<String, FileTreeEntry>) -> FileTreeNode {
    let mut root = FileTreeNode {
        name: String::new(),
        kind: FileKind::Dir,
        size: None,
        created_at: None,
        modified_at: None,
        created_by: None,
        modified_by: None,
        children: Some(Vec::new()),
    };
    // BTreeMap iterates in sorted path order, so a parent always
    // precedes its children — every insert finds its parent present.
    for (path, entry) in state {
        let components: Vec<&str> = path.split('/').collect();
        let mut node = &mut root;
        for comp in &components[..components.len() - 1] {
            let children = node.children.get_or_insert_with(Vec::new);
            // The parent must already exist (sorted order); if a
            // synthetic parent is missing, create a bare dir node.
            let idx = match children.iter().position(|c| c.name == *comp) {
                Some(i) => i,
                None => {
                    children.push(FileTreeNode {
                        name: (*comp).to_string(),
                        kind: FileKind::Dir,
                        size: None,
                        created_at: None,
                        modified_at: None,
                        created_by: None,
                        modified_by: None,
                        children: Some(Vec::new()),
                    });
                    children.len() - 1
                }
            };
            node = &mut children[idx];
        }
        let leaf = FileTreeNode {
            name: components[components.len() - 1].to_string(),
            kind: entry.kind,
            size: entry.size,
            created_at: entry.created_at,
            modified_at: entry.modified_at,
            created_by: entry.created_by.clone(),
            modified_by: entry.modified_by.clone(),
            children: if entry.kind == FileKind::Dir {
                Some(Vec::new())
            } else {
                None
            },
        };
        let children = node.children.get_or_insert_with(Vec::new);
        match children.iter().position(|c| c.name == leaf.name) {
            // A synthetic parent placeholder may already sit here;
            // replace it with the real entry, preserving its children.
            Some(i) => {
                let existing_children = children[i].children.take();
                children[i] = leaf;
                if children[i].children.is_none() {
                    children[i].children = existing_children;
                } else if let Some(existing) = existing_children {
                    children[i].children = Some(existing);
                }
            }
            None => children.push(leaf),
        }
    }
    root
}

/// Read SSE messages, fold each [`FileTreeEvent`] into `shared.state`,
/// fire the callback with the refreshed set, and bump the change
/// counter. Runs until the stream closes. Parse errors and non-message
/// events are skipped; a transport error ends the pump (the caller
/// rebuilds — reconnection is not automatic).
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                let Ok(event) = serde_json::from_str::<FileTreeEvent>(&message.data)
                else {
                    // Skip a frame we can't parse rather than tearing down.
                    continue;
                };
                let snapshot = {
                    let mut state = shared.state.lock().await;
                    apply_event(&mut state, event);
                    shared.on_change.as_ref().map(|_| Shared::entries(&state))
                };
                if let (Some(callback), Some(snapshot)) =
                    (&shared.on_change, &snapshot)
                {
                    callback(snapshot);
                }
                shared.changes.send_modify(|version| {
                    *version = version.wrapping_add(1);
                });
            }
            // Transport error (including the stream closing): stop.
            Err(_) => break,
        }
    }
}
