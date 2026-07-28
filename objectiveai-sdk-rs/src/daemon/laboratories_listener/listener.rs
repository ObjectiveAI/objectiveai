//! Materialized consumer of the cli daemon's `/laboratories/{id}`
//! endpoint.
//!
//! [`LaboratoriesListener`] connects once, then folds every
//! incoming [`LaboratoryInstanceEvent::Laboratory`] frame — always a
//! full-value record replace — into an in-memory
//! `Option<LaboratoryRecord>`.
//!
//! Ways to observe it:
//! - [`laboratory`](LaboratoriesListener::laboratory) —
//!   async snapshot of the current record (`None` before the first
//!   frame).
//! - [`subscribe`](LaboratoriesListener::subscribe) — async,
//!   blocks until the next change.
//! - [`changes`](LaboratoriesListener::changes) — the raw
//!   change-counter receiver, for race-free condition waits.
//!
//! One listener = one connection: the internal pump runs until the
//! daemon socket closes; after that the view is frozen at its last
//! state. Dropping the listener aborts the pump. Reconnection is the
//! caller's loop — mint a new listener from the client.

use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::Event;
use tokio::sync::{Mutex, watch};

use super::{LaboratoryInstanceEvent, LaboratoryRecord};
use crate::daemon::Error;

/// The shared inner state, held by both the listener handle and its
/// pump task.
struct Shared {
    /// The latest full record — `None` until the first frame lands.
    state: Mutex<Option<LaboratoryRecord>>,
    /// A monotonically-bumped change counter. Each applied frame
    /// bumps it, waking every
    /// [`subscribe`](LaboratoriesListener::subscribe) waiter.
    changes: watch::Sender<u64>,
}

/// The materialized `/laboratories/{id}` view — see the module docs.
/// Minted by
/// [`Client::laboratories_listener`](crate::daemon::Client::laboratories_listener);
/// returned only once the stream has OPENED. Dropping it aborts the
/// background pump.
pub struct LaboratoriesListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl LaboratoriesListener {
    /// Open the SSE stream (awaiting the open frame) and start the
    /// pump. The listener immediately begins folding frames (the first
    /// is the endpoint's connect-time record).
    pub(crate) async fn connect(
        client: &crate::daemon::Client,
        laboratory_id: &str,
    ) -> Result<LaboratoriesListener, Error> {
        let source = client
            .open_sse(&format!("/laboratories/{laboratory_id}"))
            .await?;
        let shared = Arc::new(Shared {
            state: Mutex::new(None),
            changes: watch::channel(0u64).0,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(LaboratoriesListener { shared, pump })
    }

    /// Snapshot the current record — `None` before the first frame.
    pub async fn laboratory(&self) -> Option<LaboratoryRecord> {
        self.shared.state.lock().await.clone()
    }

    /// Block until the next frame is applied to the state. A fresh
    /// call waits for the FIRST change after it is made — pair with a
    /// [`laboratory`](Self::laboratory) read in a loop, or hold a
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

impl Drop for LaboratoriesListener {
    fn drop(&mut self) {
        // Stop updating a view no one holds any more.
        self.pump.abort();
    }
}

/// Read frames, fold each [`LaboratoryInstanceEvent`] into
/// `shared.state`, and bump the change counter. Runs until the
/// connection closes. Parse errors and non-text frames are skipped;
/// transport errors end the pump.
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                match serde_json::from_str::<LaboratoryInstanceEvent>(&message.data) {
                    Ok(LaboratoryInstanceEvent::Laboratory { laboratory }) => {
                        {
                            let mut state = shared.state.lock().await;
                            *state = Some(laboratory);
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
