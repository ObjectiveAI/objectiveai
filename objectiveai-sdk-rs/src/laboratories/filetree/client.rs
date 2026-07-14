//! Materialized consumer of a laboratory's `/filetree` SSE endpoint.
//!
//! [`FileTree`] is NOT a raw event stream — it connects once, then
//! folds every incoming [`FileTreeEvent`] into an in-memory,
//! self-updating recursive tree (the watched root's
//! [`FileTreeNode`](super::FileTreeNode) children): a
//! [`Snapshot`](FileTreeEvent::Snapshot) replaces the whole child set,
//! [`Upserted`](FileTreeEvent::Upserted) inserts/replaces one node at
//! its path (a directory node carries its whole subtree), and
//! [`Removed`](FileTreeEvent::Removed) drops the node at its path.
//!
//! Two ways to observe it:
//! - [`tree`](FileTree::tree) — async snapshot of the current tree.
//! - an on-change **callback**
//!   ([`on_change`](FileTreeBuilder::on_change)), invoked with the full
//!   refreshed tree on every applied change.
//! - [`subscribe`](FileTree::subscribe) — async, blocks until the next
//!   change.
//!
//! One [`FileTree`] = one connection: the internal pump runs until the
//! laboratory closes the stream; after that the view is frozen at its
//! last state. Dropping it aborts the pump. Reconnection is the
//! caller's loop — build a new [`FileTree`].

use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{Mutex, watch};

use super::{FileTreeEvent, FileTreeNode};

/// The on-change callback: invoked with the full current tree (the
/// watched root's child nodes) after each applied [`FileTreeEvent`].
pub type OnChange = Box<dyn Fn(&[FileTreeNode]) + Send + Sync>;

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
    /// The tree itself: the watched root's child nodes, kept live by
    /// folding each event.
    state: Mutex<Vec<FileTreeNode>>,
    /// A monotonically-bumped change counter. Each applied event bumps
    /// it, waking every [`subscribe`](FileTree::subscribe) waiter.
    changes: watch::Sender<u64>,
    /// Optional push callback, invoked with the full tree after each
    /// change.
    on_change: Option<OnChange>,
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

    /// Register a callback invoked with the full current tree after
    /// every applied change. Runs on the pump task, so keep it cheap
    /// and non-blocking; for the tree on demand use
    /// [`tree`](FileTree::tree).
    pub fn on_change(
        mut self,
        callback: impl Fn(&[FileTreeNode]) + Send + Sync + 'static,
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
            state: Mutex::new(Vec::new()),
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

    /// Snapshot the current tree — the watched root's child nodes.
    /// Empty before the first snapshot has been folded.
    pub async fn tree(&self) -> Vec<FileTreeNode> {
        self.shared.state.lock().await.clone()
    }

    /// Block until the next change is applied. A fresh call waits for
    /// the FIRST change after it is made, so a change that lands
    /// between a preceding [`tree`](Self::tree) read and this
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

/// Fold one event into the live tree. `Snapshot` replaces the whole
/// child set; `Upserted` inserts/replaces one node at its path (a
/// directory node carries its whole subtree); `Removed` drops the node
/// at its path (and, being a subtree, everything under it).
fn apply_event(root: &mut Vec<FileTreeNode>, event: FileTreeEvent) {
    match event {
        FileTreeEvent::Snapshot { children } => {
            *root = children;
        }
        FileTreeEvent::Upserted { path, node } => {
            let Some((leaf, parents)) = path.split_last() else {
                // Empty path can't address a node — ignore.
                return;
            };
            let siblings = descend_mut(root, parents);
            match siblings.iter().position(|c| c.name() == leaf) {
                Some(i) => siblings[i] = node,
                None => siblings.push(node),
            }
        }
        FileTreeEvent::Removed { path } => {
            let Some((leaf, parents)) = path.split_last() else {
                return;
            };
            let siblings = descend_mut(root, parents);
            siblings.retain(|c| c.name() != leaf);
        }
    }
}

/// Walk `comps` from `children`, following each component into its
/// directory's child list; a missing middle segment is created as a
/// synthetic empty directory (defensive — the server sends parents
/// before children, but a dropped frame shouldn't wedge the fold).
/// Returns the child list at the end of the walk.
fn descend_mut<'a>(
    mut children: &'a mut Vec<FileTreeNode>,
    comps: &[String],
) -> &'a mut Vec<FileTreeNode> {
    for comp in comps {
        let idx = match children.iter().position(|c| c.name() == comp) {
            Some(i) => i,
            None => {
                children.push(FileTreeNode::Directory {
                    name: comp.clone(),
                    created_at: None,
                    modified_at: None,
                    created_by: None,
                    modified_by: None,
                    children: Vec::new(),
                });
                children.len() - 1
            }
        };
        // A non-directory sitting where a directory should be gets
        // replaced with an empty directory so the walk can continue.
        if children[idx].children_mut().is_none() {
            children[idx] = FileTreeNode::Directory {
                name: comp.clone(),
                created_at: None,
                modified_at: None,
                created_by: None,
                modified_by: None,
                children: Vec::new(),
            };
        }
        children = children[idx].children_mut().expect("just ensured directory");
    }
    children
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
                    shared.on_change.as_ref().map(|_| state.clone())
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
