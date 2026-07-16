//! Materialized consumer of a live file-tree SSE stream — a
//! laboratory MCP's own `/filetree` endpoint
//! ([`FileTree::laboratory`]) or a CLI daemon's
//! `/laboratories/{id}/filetree` proxy ([`FileTree::daemon`]); both
//! speak the identical wire contract.
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

/// Unconnected configuration — [`FileTree::laboratory`] /
/// [`FileTree::daemon`] + builder methods + [`FileTreeBuilder::connect`].
pub struct FileTreeBuilder {
    /// The full endpoint URL — `{lab}/filetree` (laboratory variant) or
    /// `{daemon}/laboratories/{id}/filetree` (daemon variant).
    url: String,
    /// Query params to send (the laboratory variant's `path` scope, the
    /// daemon variant's host pin).
    query: Vec<(String, String)>,
    /// Optional daemon auth signature, sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` header (daemon variant only —
    /// laboratories have no auth).
    signature: Option<String>,
    /// Optional on-change callback.
    on_change: Option<OnChange>,
}

impl FileTreeBuilder {
    /// Scope the watch to an absolute path inside the container.
    /// Laboratory variant only: the daemon endpoint always serves the
    /// laboratory's workspace (its cwd) and ignores this.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.query.push(("path".to_string(), path.into()));
        self
    }

    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` header. Daemon variant only; without
    /// it the daemon must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Pin the exact serving host (`?machine=…&machine_state=…`) —
    /// laboratory ids are only unique per (machine, state). Daemon
    /// variant only; omit for the legacy first-match-by-id scan.
    pub fn host(
        mut self,
        machine: impl Into<String>,
        machine_state: impl Into<String>,
    ) -> Self {
        self.query.push(("machine".to_string(), machine.into()));
        self
            .query
            .push(("machine_state".to_string(), machine_state.into()));
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
    /// the endpoint's connect-time snapshot).
    pub async fn connect(self) -> Result<FileTree, Error> {
        let client = reqwest::Client::builder().build()?;
        let mut request = client
            .get(&self.url)
            .header("Accept", "text/event-stream");
        if !self.query.is_empty() {
            request = request.query(&self.query);
        }
        if let Some(signature) = &self.signature {
            request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
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
/// via [`FileTree::laboratory`] or [`FileTree::daemon`]. Dropping it
/// aborts the background pump.
pub struct FileTree {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl FileTree {
    /// Build a [`FileTree`] against a laboratory MCP's own `/filetree`
    /// endpoint, from its base HTTP address (e.g.
    /// `http://127.0.0.1:<port>` — the loopback-published container
    /// port). No auth — the port itself is the trust boundary.
    pub fn laboratory(base_url: impl Into<String>) -> FileTreeBuilder {
        FileTreeBuilder {
            url: format!("{}/filetree", base_url.into()),
            query: Vec::new(),
            signature: None,
            on_change: None,
        }
    }

    /// Build a [`FileTree`] against a CLI daemon's
    /// `/laboratories/{id}/filetree` endpoint, from the daemon's
    /// published `http://` address and the RAW laboratory id (ids
    /// forbid `/`, so the URL is safe by construction). Same wire
    /// contract as the laboratory variant — the daemon re-emits the
    /// identical events from its own materialized state. Attach
    /// [`signature`](FileTreeBuilder::signature) against a secretful
    /// daemon and [`host`](FileTreeBuilder::host) to pin the exact
    /// serving host.
    pub fn daemon(
        base_url: impl Into<String>,
        laboratory_id: impl AsRef<str>,
    ) -> FileTreeBuilder {
        FileTreeBuilder {
            url: format!(
                "{}/laboratories/{}/filetree",
                base_url.into(),
                laboratory_id.as_ref(),
            ),
            query: Vec::new(),
            signature: None,
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
                    event.apply(&mut state);
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
