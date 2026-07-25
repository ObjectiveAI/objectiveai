//! Materialized consumer of the cli daemon's `/channels` endpoint —
//! the OFFER lifecycle side of duplex channels.
//!
//! [`ChannelListener`] connects once, then folds every incoming
//! [`ChannelEvent`]: [`Offer`](ChannelEvent::Offer) inserts into the
//! pending-offers map, [`OfferWithdrawn`](ChannelEvent::OfferWithdrawn)
//! removes.
//!
//! Ways to observe it:
//! - [`pending`](ChannelListener::pending) — async snapshot of the
//!   open offers (sorted by channel id).
//! - [`subscribe`](ChannelListener::subscribe) — async, blocks until
//!   the next applied event.
//! - [`changes`](ChannelListener::changes) — the raw change-counter
//!   receiver, for race-free condition waits.
//!
//! Accepting is
//! [`Client::accept_channel`](crate::daemon::Client::accept_channel):
//! a bare `POST /channels/{id}/accept` (first-wins) whose `200` body
//! carries the owner secret (`S_owner`) — the per-channel capability
//! for `channels logs reply|list|open|subscribe` and
//! `channels close`. The daemon tracks no liveness; a channel stays
//! open until someone runs `channels close` with either of its
//! secrets.
//!
//! One listener = one connection: the internal pump runs until the
//! daemon socket closes; after that the view is frozen. Dropping the
//! listener aborts the pump. Reconnection is the caller's loop — mint
//! a new listener from the client.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::Event;
use tokio::sync::{Mutex, watch};

use super::{ChannelEvent, ChannelOffer};
use crate::daemon::Error;

/// The shared inner state, held by both the listener handle and its
/// pump task.
struct Shared {
    /// `channel_id → offer` — the currently OPEN offers.
    offers: Mutex<BTreeMap<String, ChannelOffer>>,
    /// Monotonically-bumped event counter; each applied event bumps
    /// it, waking every [`subscribe`](ChannelListener::subscribe)
    /// waiter.
    changes: watch::Sender<u64>,
}

/// The materialized `/channels` offer view — see the module docs.
/// Minted by
/// [`Client::channel_listener`](crate::daemon::Client::channel_listener);
/// returned only once the stream has OPENED. Dropping it aborts the
/// background pump.
pub struct ChannelListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl ChannelListener {
    /// Open the SSE stream (awaiting the open frame) and start the
    /// pump. The daemon replays every open offer first, so the view
    /// converges immediately.
    pub(crate) async fn connect(
        client: &crate::daemon::Client,
    ) -> Result<ChannelListener, Error> {
        let source = client.open_sse("/channels").await?;
        let shared = Arc::new(Shared {
            offers: Mutex::new(BTreeMap::new()),
            changes: watch::channel(0u64).0,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(ChannelListener { shared, pump })
    }

    /// Snapshot the currently open offers, sorted by channel id.
    pub async fn pending(&self) -> Vec<ChannelOffer> {
        self.shared.offers.lock().await.values().cloned().collect()
    }

    /// Block until the next event is applied. Pair with
    /// [`pending`](Self::pending) in a loop, or hold a
    /// [`changes`](Self::changes) receiver for race-free waits.
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

impl Drop for ChannelListener {
    fn drop(&mut self) {
        // Stop updating a view no one holds any more.
        self.pump.abort();
    }
}

/// Fold one event into the shared state.
async fn apply_event(shared: &Shared, event: &ChannelEvent) {
    match event {
        ChannelEvent::Offer { offer } => {
            shared
                .offers
                .lock()
                .await
                .insert(offer.channel_id.clone(), offer.clone());
        }
        ChannelEvent::OfferWithdrawn { channel_id } => {
            shared.offers.lock().await.remove(channel_id);
        }
        ChannelEvent::Live => {}
    }
}

/// Read frames, fold each [`ChannelEvent`], bump the change counter.
/// Runs until the connection closes. Parse errors and non-text frames
/// are skipped; transport errors end the pump.
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                match serde_json::from_str::<ChannelEvent>(&message.data) {
                    Ok(event) => {
                        apply_event(&shared, &event).await;
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
