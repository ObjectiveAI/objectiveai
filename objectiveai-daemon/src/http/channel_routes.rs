//! The daemon's `/channels` endpoints — DUPLEX CHANNELS: a publisher
//! offers a channel, the first client to OPEN the channel's stream
//! ACCEPTS and owns it, and the two exchange messages over a durable
//! per-channel log (`channels logs …`, backed by
//! [`crate::db::channels`]).
//!
//! [`ChannelHub`] holds only the LIVE, in-memory coordination — the
//! durable channel record + message log live in Postgres. Its state:
//!
//! - **connections** — every `GET /channels` stream (the OFFER
//!   lifecycle feed), keyed by a hub id. No secrets: the base stream
//!   carries offers, withdrawals, and the `live` marker, nothing else.
//! - **offers** — the PENDING (pre-accept) offers, keyed by channel
//!   id. An offer carries everything needed to persist the channel on
//!   accept, plus the arbitration oneshot the blocked `channels
//!   publish` command awaits and the audience set for its withdrawal.
//!
//! `POST /channels/{id}/accept` (no body) is the ACCEPT: first-wins
//! over a pending offer, answering `200` with [`ChannelAccepted`]
//! (the owner secret) or `404`/`409`. The daemon does NO liveness
//! tracking — a channel stays open until someone runs
//! `channels close` with either of its secrets (terminal; the
//! `channel_closed` NOTIFY unblocks any parked `logs subscribe`).
//!
//! Secret flow: `S_pub` is minted at offer time and returned to the
//! publisher's command; `S_owner` is minted on accept and returned in
//! the accept response. Both are bearer capabilities for the
//! per-channel log commands and `channels close`.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::response::sse::{Event, Sse};
use dashmap::DashMap;
use objectiveai_sdk::cli::channel_listener::{
    ChannelAccepted, ChannelEvent, ChannelOffer,
};
use objectiveai_sdk::identity::Identity;
use tokio::sync::{mpsc, oneshot};

/// One PENDING (pre-accept) channel offer.
struct Offer {
    channel_id: String,
    /// The publisher's per-channel capability (`S_pub`).
    pub_secret: String,
    /// Pre-serialized [`ChannelEvent::Offer`] frame — sent to every
    /// current and future connection until the offer is taken.
    offer_frame: String,
    // The offer payload, persisted verbatim when the channel is
    // accepted. The identity's plugin trio IS the publishing plugin.
    key: String,
    details: serde_json::Value,
    message: String,
    identity: Identity,
    /// Accept arbitration AND the publish unblock: the first accept
    /// takes the sender; a taken (`None`) slot means already accepted.
    accept: std::sync::Mutex<Option<oneshot::Sender<()>>>,
    /// Connection ids this offer was delivered to — the exact audience
    /// for its withdrawal notice.
    offered_to: std::sync::Mutex<HashSet<u64>>,
}

impl Default for ChannelHub {
    fn default() -> Self {
        Self::new()
    }
}

/// Why an accept failed.
pub enum AcceptError {
    /// No pending offer AND no channel row — unknown or withdrawn id.
    NotFound,
    /// The offer is gone but the channel exists (open or closed) —
    /// someone else already accepted.
    AlreadyAccepted,
    /// Persisting the channel failed.
    Db(crate::db::Error),
}

/// The channels hub — see the module docs. Clone-shared.
#[derive(Clone)]
pub struct ChannelHub {
    connections: Arc<DashMap<u64, mpsc::UnboundedSender<String>>>,
    next_connection_id: Arc<AtomicU64>,
    offers: Arc<DashMap<String, Arc<Offer>>>,
}

impl ChannelHub {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            next_connection_id: Arc::new(AtomicU64::new(1)),
            offers: Arc::new(DashMap::new()),
        }
    }

    /// Register a `GET /channels` connection: allocate its id, replay
    /// every open offer, then send [`ChannelEvent::Live`].
    fn register_connection(&self) -> (u64, mpsc::UnboundedReceiver<String>) {
        let id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        self.connections.insert(id, tx.clone());
        // Replay every open offer to the fresh connection.
        for offer in self.offers.iter() {
            let mut offered_to = offer.offered_to.lock().expect("offered_to lock");
            if offered_to.insert(id) {
                let _ = tx.send(offer.offer_frame.clone());
            }
        }
        let _ = tx.send(frame(&ChannelEvent::Live));
        (id, rx)
    }

    /// Drop a closed connection. Stale ids left in offers'
    /// `offered_to` sets are harmless — their sends go nowhere.
    fn unregister_connection(&self, id: u64) {
        self.connections.remove(&id);
    }

    /// Create a pending offer and fan it out to every current
    /// connection. Returns `(channel_id, S_pub, accept_rx)` — the
    /// publisher's command holds `S_pub` and awaits `accept_rx`.
    pub fn create_offer(
        &self,
        key: String,
        details: serde_json::Value,
        message: String,
        identity: Identity,
    ) -> (String, String, oneshot::Receiver<()>) {
        let channel_id = uuid::Uuid::new_v4().to_string();
        let pub_secret = uuid::Uuid::new_v4().to_string();
        let offer_frame = frame(&ChannelEvent::Offer {
            offer: ChannelOffer {
                channel_id: channel_id.clone(),
                identity: identity.clone(),
                key: key.clone(),
                details: details.clone(),
                message: message.clone(),
            },
        });
        let (accept_tx, accept_rx) = oneshot::channel();
        let offer = Arc::new(Offer {
            channel_id: channel_id.clone(),
            pub_secret: pub_secret.clone(),
            offer_frame,
            key,
            details,
            message,
            identity,
            accept: std::sync::Mutex::new(Some(accept_tx)),
            offered_to: std::sync::Mutex::new(HashSet::new()),
        });
        self.offers.insert(channel_id.clone(), Arc::clone(&offer));
        for connection in self.connections.iter() {
            let mut offered_to = offer.offered_to.lock().expect("offered_to lock");
            if offered_to.insert(*connection.key()) {
                let _ = connection.value().send(offer.offer_frame.clone());
            }
        }
        (channel_id, pub_secret, accept_rx)
    }

    /// End an offer that was NEVER accepted (publish timed out /
    /// cancelled): drop it and send [`ChannelEvent::OfferWithdrawn`] to
    /// the connections that saw it. Idempotent; an accepted offer is
    /// already gone, so this is a no-op.
    pub fn abandon_offer(&self, channel_id: &str) {
        let Some((_, offer)) = self.offers.remove(channel_id) else {
            return;
        };
        self.notify_offered(&offer, &ChannelEvent::OfferWithdrawn {
            channel_id: channel_id.to_string(),
        });
    }

    /// The first-wins ACCEPT: arbitrate the offer's oneshot → mint
    /// `S_owner` → persist the channel → unblock the publisher →
    /// withdraw the offer from every connection that saw it. Returns
    /// `S_owner` — the accept response's body.
    pub async fn accept(
        &self,
        pool: &crate::db::Pool,
        channel_id: &str,
    ) -> Result<String, AcceptError> {
        let Some(offer) = self.offers.get(channel_id).map(|e| Arc::clone(e.value()))
        else {
            // No pending offer: an existing channel row means someone
            // already accepted; nothing at all means unknown/withdrawn.
            return match crate::db::channels::channel_state(pool, channel_id).await {
                Ok(Some(_)) => Err(AcceptError::AlreadyAccepted),
                Ok(None) => Err(AcceptError::NotFound),
                Err(e) => Err(AcceptError::Db(e)),
            };
        };
        // Arbitrate: the first accept takes the oneshot. Take it out
        // under the sync lock, then do all async work lock-free.
        let winner = {
            let mut slot = offer.accept.lock().expect("accept lock");
            match slot.take() {
                Some(sender) => sender,
                None => return Err(AcceptError::AlreadyAccepted),
            }
        };
        let owner_secret = uuid::Uuid::new_v4().to_string();
        // Persist the channel BEFORE unblocking publish — if this
        // fails, `winner` drops without firing, so publish sees the
        // offer as abandoned rather than succeeding against no row.
        if let Err(e) = crate::db::channels::insert_channel(
            pool,
            channel_id,
            &offer.pub_secret,
            &owner_secret,
            &offer.key,
            &offer.details,
            &offer.message,
            &crate::db::channels::PluginOrigin {
                owner: offer.identity.plugin_owner.as_deref(),
                name: offer.identity.plugin_name.as_deref(),
                version: offer.identity.plugin_version.as_deref(),
            },
            &offer.identity,
        )
        .await
        {
            return Err(AcceptError::Db(e));
        }
        // The offer is consumed.
        self.offers.remove(channel_id);
        // Unblock the publisher's command (the row exists — it may
        // immediately write requests).
        let _ = winner.send(());
        // Every connection that saw the offer learns it's gone — the
        // accepter's own base listener included (correct: the offer IS
        // gone; the fold drops it from the pending map).
        self.notify_offered(&offer, &ChannelEvent::OfferWithdrawn {
            channel_id: channel_id.to_string(),
        });
        Ok(owner_secret)
    }

    /// Send one event to exactly the connections that saw `offer`.
    fn notify_offered(&self, offer: &Offer, event: &ChannelEvent) {
        let payload = frame(event);
        let offered_to = offer.offered_to.lock().expect("offered_to lock");
        for id in offered_to.iter() {
            if let Some(connection) = self.connections.get(id) {
                let _ = connection.send(payload.clone());
            }
        }
    }
}

/// Serialize one [`ChannelEvent`] to its SSE frame string.
fn frame(event: &ChannelEvent) -> String {
    serde_json::to_string(event).expect("ChannelEvent serializes")
}

/// RAII on the base `GET /channels` stream: dropping it unregisters
/// the connection. Nothing else — the base stream owns no channels.
struct ConnectionGuard {
    hub: ChannelHub,
    id: u64,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.hub.unregister_connection(self.id);
    }
}

/// `GET /channels`: header-auth, then an SSE stream of
/// [`ChannelEvent`]s — offer replay, the `live` marker, then live
/// offers and withdrawals. No DB involved.
pub(crate) async fn channels_handler(
    axum::extract::State(state): axum::extract::State<
        crate::http::daemon_stream::DaemonHttpState,
    >,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !crate::http::daemon_auth::authenticate_header(
        &headers,
        state.global.auth_secret().as_ref(),
    ) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    let hub = state.channels.clone();
    let (id, mut rx) = hub.register_connection();
    let stream = async_stream::stream! {
        let _guard = ConnectionGuard { hub, id };
        while let Some(frame) = rx.recv().await {
            yield Ok::<_, std::convert::Infallible>(Event::default().data(frame));
        }
    };
    Sse::new(stream).into_response()
}

/// `POST /channels/{id}/accept`: header-auth, no body. First-wins
/// accept of a pending offer ([`ChannelHub::accept`]). `200` answers
/// with [`ChannelAccepted`] — the owner secret (`S_owner`); `404` =
/// unknown or withdrawn id; `409` = already accepted; `500` = DB.
pub(crate) async fn channels_accept_handler(
    axum::extract::State(state): axum::extract::State<
        crate::http::daemon_stream::DaemonHttpState,
    >,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !crate::http::daemon_auth::authenticate_header(
        &headers,
        state.global.auth_secret().as_ref(),
    ) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    let pool = match state.global.db_client().await {
        Ok(pool) => pool,
        Err(e) => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                format!("channels db: {e}"),
            )
                .into_response();
        }
    };
    match state.channels.accept(&pool, &id).await {
        Ok(secret) => {
            let body = serde_json::to_string(&ChannelAccepted { secret })
                .expect("ChannelAccepted serializes");
            (axum::http::StatusCode::OK, body).into_response()
        }
        Err(AcceptError::NotFound) => axum::http::StatusCode::NOT_FOUND.into_response(),
        Err(AcceptError::AlreadyAccepted) => {
            axum::http::StatusCode::CONFLICT.into_response()
        }
        Err(AcceptError::Db(e)) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("channel accept: {e}"),
        )
            .into_response(),
    }
}
