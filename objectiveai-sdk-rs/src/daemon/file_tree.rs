//! Materialized consumer of a live file-tree SSE stream — a
//! laboratory MCP's own `/filetree` endpoint
//! ([`FileTree::laboratory`]) or a CLI daemon's
//! `/laboratories/{id}/filetree` proxy
//! ([`Client::file_tree`](crate::daemon::Client::file_tree)); both
//! speak the identical wire contract.
//!
//! [`FileTree`] is NOT a raw event stream — it connects once, then
//! folds every incoming [`FileTreeEvent`] into an in-memory,
//! self-updating recursive tree (the watched root's
//! [`FileTreeNode`] children): a
//! [`Snapshot`](FileTreeEvent::Snapshot) replaces the whole child set,
//! [`Upserted`](FileTreeEvent::Upserted) inserts/replaces one node at
//! its path (a directory node carries its whole subtree), and
//! [`Removed`](FileTreeEvent::Removed) drops the node at its path.
//!
//! Ways to observe it:
//! - [`tree`](FileTree::tree) — async snapshot of the current tree.
//! - [`subscribe`](FileTree::subscribe) — async, blocks until the next
//!   change.
//! - [`changes`](FileTree::changes) — the raw change-counter receiver,
//!   for race-free condition waits.
//!
//! The machine/machine_state query the daemon route accepts is not
//! surfaced here (no consumer); add when needed.
//!
//! One [`FileTree`] = one connection: the internal pump runs until the
//! laboratory closes the stream; after that the view is frozen at its
//! last state. Dropping it aborts the pump. Reconnection is the
//! caller's loop — mint a new [`FileTree`].

use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{Mutex, watch};

use crate::daemon::Error;
use crate::laboratories::filetree::{FileTreeEvent, FileTreeNode};

/// The shared inner state, held by both the [`FileTree`] handle and
/// its pump task.
struct Shared {
    /// The tree itself: the watched root's child nodes, kept live by
    /// folding each event.
    state: Mutex<Vec<FileTreeNode>>,
    /// A monotonically-bumped change counter. Each applied event bumps
    /// it, waking every [`subscribe`](FileTree::subscribe) waiter.
    changes: watch::Sender<u64>,
}

/// The materialized `/filetree` view — see the module docs. Minted by
/// [`Client::file_tree`](crate::daemon::Client::file_tree) (daemon
/// flavor) or [`FileTree::laboratory`]; returned only once the stream
/// has OPENED. Dropping it aborts the background pump.
pub struct FileTree {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl FileTree {
    /// The daemon flavor — minted by
    /// [`Client::file_tree`](crate::daemon::Client::file_tree). Opens
    /// the daemon's `/laboratories/{id}/filetree` proxy (the RAW
    /// laboratory id — ids forbid `/`, so the URL is safe by
    /// construction), awaiting the open frame, and starts the pump.
    /// Same wire contract as the laboratory flavor — the daemon
    /// re-emits the identical events from its own materialized state,
    /// always serving the laboratory's workspace (its cwd).
    pub(crate) async fn connect_daemon(
        client: &crate::daemon::Client,
        laboratory_id: &str,
    ) -> Result<FileTree, Error> {
        let source = client
            .open_sse(&format!("/laboratories/{laboratory_id}/filetree"))
            .await?;
        Ok(Self::start(source))
    }

    /// The LABORATORY flavor: dial a laboratory's OWN `/filetree` (no
    /// auth — loopback-only lab server), optionally scoped to `path`
    /// (an absolute path inside the container), from its base HTTP
    /// address (e.g. `http://127.0.0.1:<port>` — the loopback-published
    /// container port; the port itself is the trust boundary). Awaits
    /// the open frame, then starts the pump.
    pub async fn laboratory(
        base_url: &str,
        path: Option<&str>,
    ) -> Result<FileTree, Error> {
        let mut request = reqwest::Client::new()
            .get(format!("{}/filetree", base_url.trim_end_matches('/')))
            .header("Accept", "text/event-stream");
        if let Some(path) = path {
            request = request.query(&[("path", path)]);
        }
        let mut source = request.eventsource()?;
        // reqwest-eventsource yields `Open` first on a successful
        // response; an error (invalid status included) or immediate
        // end means the stream never opened.
        match source.next().await {
            Some(Ok(Event::Open)) => {}
            Some(Err(e)) => return Err(Error::Open(e)),
            Some(Ok(Event::Message(_))) | None => return Err(Error::Closed),
        }
        Ok(Self::start(source))
    }

    /// Build the shared state and spawn the pump over an
    /// already-opened stream.
    fn start(source: reqwest_eventsource::EventSource) -> FileTree {
        let shared = Arc::new(Shared {
            state: Mutex::new(Vec::new()),
            changes: watch::channel(0u64).0,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        FileTree { shared, pump }
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
    /// hold a [`changes`](Self::changes) receiver for race-free waits.
    pub async fn subscribe(&self) {
        let mut rx = self.shared.changes.subscribe();
        let _ = rx.changed().await;
    }

    /// The raw change-counter receiver — for RACE-FREE condition
    /// waits: hold ONE receiver across iterations of a
    /// check-then-await loop, and an event landing between the check
    /// and the await still resolves the next `changed()`.
    pub fn changes(&self) -> watch::Receiver<u64> {
        self.shared.changes.subscribe()
    }
}

impl Drop for FileTree {
    fn drop(&mut self) {
        self.pump.abort();
    }
}

/// Read SSE messages, fold each [`FileTreeEvent`] into `shared.state`,
/// and bump the change counter. Runs until the stream closes. Parse
/// errors and non-message events are skipped; a transport error ends
/// the pump (the caller rebuilds — reconnection is not automatic).
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
                {
                    let mut state = shared.state.lock().await;
                    event.apply(&mut state);
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
