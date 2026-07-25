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
//! Ways to observe it:
//! - [`laboratories`](LaboratoriesListListener::laboratories)
//!   — async snapshot of the current set (sorted by id).
//! - [`subscribe`](LaboratoriesListListener::subscribe) —
//!   async, blocks until the next change.
//! - [`changes`](LaboratoriesListListener::changes) — the raw
//!   change-counter receiver, for race-free condition waits.
//!
//! One listener = one connection: the internal pump runs until the
//! daemon socket closes; after that the view is frozen at its last
//! state. Dropping the listener aborts the pump. Reconnection (the
//! daemon's address changes across restarts) is the caller's loop —
//! mint a new listener from the client.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::Event;
use tokio::sync::{Mutex, watch};

use super::{LaboratoryEvent, LaboratoryStatus};
use crate::daemon::Error;

/// The shared inner state, held by both the listener handle and its
/// pump task.
struct Shared {
    /// `(machine, state, id) fold key → status` (see [`fold_key`] —
    /// laboratory ids are only unique per (machine, state)). A
    /// `BTreeMap` so iteration (snapshots) is sorted by id.
    state: Mutex<BTreeMap<String, LaboratoryStatus>>,
    /// A monotonically-bumped change counter. Each applied event bumps
    /// it, waking every
    /// [`subscribe`](LaboratoriesListListener::subscribe)
    /// waiter.
    changes: watch::Sender<u64>,
}

impl Shared {
    fn statuses(state: &BTreeMap<String, LaboratoryStatus>) -> Vec<LaboratoryStatus> {
        let mut statuses: Vec<LaboratoryStatus> = state.values().cloned().collect();
        statuses.sort_by(|a, b| a.id.cmp(&b.id));
        statuses
    }
}

/// The materialized `/laboratories/list` view — see the module docs.
/// Minted by
/// [`Client::laboratories_list_listener`](crate::daemon::Client::laboratories_list_listener);
/// returned only once the stream has OPENED. Dropping it aborts the
/// background pump.
pub struct LaboratoriesListListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl LaboratoriesListListener {
    /// Open the SSE stream (awaiting the open frame) and start the
    /// pump. The listener immediately begins folding events (the first
    /// is the endpoint's connect-time snapshot).
    pub(crate) async fn connect(
        client: &crate::daemon::Client,
    ) -> Result<LaboratoriesListListener, Error> {
        let source = client.open_sse("/laboratories/list").await?;
        let shared = Arc::new(Shared {
            state: Mutex::new(BTreeMap::new()),
            changes: watch::channel(0u64).0,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(LaboratoriesListListener { shared, pump })
    }

    /// Snapshot the current laboratory set, sorted by id.
    pub async fn laboratories(&self) -> Vec<LaboratoryStatus> {
        Shared::statuses(&*self.shared.state.lock().await)
    }

    /// Block until the next change is applied to the state. A fresh
    /// call waits for the FIRST change after it is made, so a change
    /// that lands between a preceding
    /// [`laboratories`](Self::laboratories) read and this call is not
    /// observed by it — pair with the read in a loop, or hold a
    /// [`changes`](Self::changes) receiver for race-free waits.
    pub async fn subscribe(&self) {
        // A receiver from `subscribe` is caught up to the current
        // version, so `changed` resolves on the next bump. `Err` only
        // if the sender dropped (pump gone) — treat as "no more
        // changes" and return.
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

/// Read frames, fold each `LaboratoryEvent` into `shared.state`, and
/// bump the change counter. Runs until the connection closes. Parse
/// errors and non-text frames are skipped; transport errors end the
/// pump.
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                match serde_json::from_str::<LaboratoryEvent>(&message.data) {
                    Ok(event) => {
                        {
                            let mut state = shared.state.lock().await;
                            apply_event(&mut state, event);
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
