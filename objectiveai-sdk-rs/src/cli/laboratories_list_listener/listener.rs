//! Materialized consumer of the cli daemon's `/laboratories/list`
//! endpoint.
//!
//! [`LaboratoriesListListener`] is NOT a raw event stream —
//! it connects once, then folds every incoming [`LaboratoryEvent`]
//! into an in-memory, self-updating map of `id → LaboratoryStatus`: a
//! [`Snapshot`](LaboratoryEvent::Snapshot) replaces the whole set,
//! [`Upserted`](LaboratoryEvent::Upserted) replaces one laboratory by
//! id (introducing it if unseen), and
//! [`Removed`](LaboratoryEvent::Removed) drops one. Per-lab attachment
//! detail is `/laboratories/{id}`'s job.
//!
//! Three ways to observe it:
//! - [`laboratories`](LaboratoriesListListener::laboratories)
//!   — async snapshot of the current set (sorted by id).
//! - an on-change **callback**
//!   ([`on_change`](LaboratoriesListListenerBuilder::on_change)),
//!   invoked with the full refreshed set on every applied change.
//! - [`subscribe`](LaboratoriesListListener::subscribe) —
//!   async, blocks until the next change.
//!
//! One listener = one connection: the internal pump runs until the
//! daemon socket closes; after that the view is frozen at its last
//! state. Dropping the listener aborts the pump. Reconnection (the
//! daemon's address changes across restarts) is the caller's loop —
//! build a new listener.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{Mutex, watch};

use super::{LaboratoryEvent, LaboratoryStatus};

/// The on-change callback: invoked with the full current laboratory
/// set (sorted by id) after each applied [`LaboratoryEvent`].
pub type OnChange = Box<dyn Fn(&[LaboratoryStatus]) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request builder rejected the URL, or opening the SSE
    /// stream failed.
    #[error("connect daemon sse: {0}")]
    Connect(#[from] reqwest_eventsource::CannotCloneRequestError),
    /// The underlying reqwest client failed to build.
    #[error("daemon sse http client: {0}")]
    Client(#[from] reqwest::Error),
}

/// The shared inner state, held by both the listener handle and its
/// pump task.
struct Shared {
    /// `(machine, state, id) fold key → status` (see [`fold_key`] —
    /// laboratory ids are only unique per (machine, state)). A
    /// `BTreeMap` so iteration (snapshots, the
    /// callback) is sorted by id.
    state: Mutex<BTreeMap<String, LaboratoryStatus>>,
    /// A monotonically-bumped change counter. Each applied event bumps
    /// it, waking every
    /// [`subscribe`](LaboratoriesListListener::subscribe)
    /// waiter.
    changes: watch::Sender<u64>,
    /// Optional push callback, invoked with the full set after each
    /// change.
    on_change: Option<OnChange>,
}

impl Shared {
    fn statuses(state: &BTreeMap<String, LaboratoryStatus>) -> Vec<LaboratoryStatus> {
        let mut statuses: Vec<LaboratoryStatus> = state.values().cloned().collect();
        statuses.sort_by(|a, b| a.id.cmp(&b.id));
        statuses
    }
}

/// Unconnected configuration —
/// [`LaboratoriesListListener::new`] +
/// [`LaboratoriesListListenerBuilder::signature`] +
/// [`LaboratoriesListListenerBuilder::connect`].
pub struct LaboratoriesListListenerBuilder {
    /// Full connect URL of the daemon's laboratories route, e.g.
    /// `http://127.0.0.1:49152/laboratories/list`.
    url: String,
    /// Optional auth signature, sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header.
    signature: Option<String>,
    /// Optional on-change callback.
    on_change: Option<OnChange>,
}

impl LaboratoriesListListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header — the daemon's SSE
    /// watcher routes authenticate by header. Without it the daemon
    /// must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Register a callback invoked with the full current laboratory
    /// set (sorted by id) after every applied change. Runs on the
    /// pump task, so keep it cheap and non-blocking; for the full
    /// state on demand use
    /// [`laboratories`](LaboratoriesListListener::laboratories).
    pub fn on_change(
        mut self,
        callback: impl Fn(&[LaboratoryStatus]) + Send + Sync + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Open the SSE stream and start the pump. The
    /// returned [`LaboratoriesListListener`] immediately
    /// begins folding events (the first is the endpoint's
    /// connect-time snapshot).
    pub async fn connect(self) -> Result<LaboratoriesListListener, Error> {
        let source = connect_sse(&self.url, self.signature.as_deref())?;

        let shared = Arc::new(Shared {
            state: Mutex::new(BTreeMap::new()),
            changes: watch::channel(0u64).0,
            on_change: self.on_change,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(LaboratoriesListListener { shared, pump })
    }
}

/// The materialized `/laboratories/list` view — see the module docs.
/// Construct via [`LaboratoriesListListener::new`]. Dropping
/// it aborts the background pump.
pub struct LaboratoriesListListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl LaboratoriesListListener {
    /// Start building a listener for the daemon's `/laboratories/list`
    /// URL (the daemon's published base address + `/laboratories/list`).
    pub fn new(url: impl Into<String>) -> LaboratoriesListListenerBuilder {
        LaboratoriesListListenerBuilder {
            url: url.into(),
            signature: None,
            on_change: None,
        }
    }

    /// Snapshot the current laboratory set, sorted by id.
    pub async fn laboratories(&self) -> Vec<LaboratoryStatus> {
        Shared::statuses(&*self.shared.state.lock().await)
    }

    /// Block until the next change is applied to the state. A fresh
    /// call waits for the FIRST change after it is made, so a change
    /// that lands between a preceding
    /// [`laboratories`](Self::laboratories) read and this call is not
    /// observed by it — pair with the read in a loop, or use the
    /// [`on_change`](LaboratoriesListListenerBuilder::on_change)
    /// callback for guaranteed push.
    pub async fn subscribe(&self) {
        // A receiver from `subscribe` is caught up to the current
        // version, so `changed` resolves on the next bump. `Err` only
        // if the sender dropped (pump gone) — treat as "no more
        // changes" and return.
        let mut rx = self.shared.changes.subscribe();
        let _ = rx.changed().await;
    }
}

impl Drop for LaboratoriesListListener {
    fn drop(&mut self) {
        // Stop updating a view no one holds any more.
        self.pump.abort();
    }
}

/// The fold key: laboratory ids are only unique per (machine, state),
/// so the map keys on the full `(machine id, machine state, id)`
/// triple — same-id laboratories from different hosts coexist. Empty
/// segments when the daemon didn't report the pair.
fn fold_key(
    id: &str,
    machine: Option<&str>,
    machine_state: Option<&str>,
) -> String {
    format!(
        "{}\n{}\n{}",
        machine.unwrap_or(""),
        machine_state.unwrap_or(""),
        id
    )
}

/// Fold one event into the map. `Snapshot` replaces; `Upserted`
/// replaces one laboratory by its (machine, state, id) key
/// (introducing it if unseen); `Removed` drops one by the same key.
fn apply_event(state: &mut BTreeMap<String, LaboratoryStatus>, event: LaboratoryEvent) {
    match event {
        LaboratoryEvent::Snapshot { laboratories } => {
            state.clear();
            for laboratory in laboratories {
                let key = fold_key(
                    &laboratory.id,
                    laboratory.machine.as_ref().map(|m| m.id.as_str()),
                    laboratory.machine_state.as_deref(),
                );
                state.insert(key, laboratory);
            }
        }
        LaboratoryEvent::Upserted { laboratory } => {
            let key = fold_key(
                &laboratory.id,
                laboratory.machine.as_ref().map(|m| m.id.as_str()),
                laboratory.machine_state.as_deref(),
            );
            state.insert(key, laboratory);
        }
        LaboratoryEvent::Removed { id, machine, machine_state } => {
            state.remove(&fold_key(&id, machine.as_deref(), machine_state.as_deref()));
        }
    }
}

/// Read frames, fold each `LaboratoryEvent` into `shared.state`, fire
/// the callback with the refreshed set, and bump the change counter.
/// Runs until the connection closes. Parse errors and non-text frames
/// are skipped; transport errors end the pump.
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                match serde_json::from_str::<LaboratoryEvent>(&message.data) {
                    Ok(event) => {
                        let snapshot = {
                            let mut state = shared.state.lock().await;
                            apply_event(&mut state, event);
                            shared.on_change.as_ref().map(|_| Shared::statuses(&state))
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
                    // Skip a frame we can't parse rather than tearing down.
                    Err(_) => continue,
                }
            }
            Err(_) => break,
        }
    }
}


/// Open the daemon's SSE watcher stream: request
/// `text/event-stream`, and stamp `X-OBJECTIVEAI-SIGNATURE` when a
/// signature is present (the daemon's watcher routes moved auth from
/// the first-frame preamble to this header).
fn connect_sse(
    url: &str,
    signature: Option<&str>,
) -> Result<reqwest_eventsource::EventSource, Error> {
    let client = reqwest::Client::builder().build()?;
    let mut request = client
        .get(url)
        .header("Accept", "text/event-stream");
    if let Some(signature) = signature {
        request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
    }
    Ok(request.eventsource()?)
}
