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
//! `GET /channels/{id}` is the PER-CHANNEL stream:
//! - **Accept** (no `X-OBJECTIVEAI-CHANNEL-SECRET` header): opening a
//!   PENDING offer's stream IS the accept (first-wins). The first
//!   frame delivers `S_owner`; the stream is the channel's LIVENESS
//!   ANCHOR — its drop closes the channel (terminal).
//! - **Observer** (header = `S_pub` or `S_owner`): silent until the
//!   channel closes, then one `closed` frame and the stream ends.
//!   Observer drops close nothing.
//!
//! Secret flow: `S_pub` is minted at offer time and returned to the
//! publisher's command; `S_owner` is minted on accept and delivered as
//! the accepting stream's FIRST frame — capability and channel life
//! ride the same connection by construction.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::response::sse::{Event, Sse};
use dashmap::DashMap;
use objectiveai_sdk::cli::channel_listener::{
    ChannelEvent, ChannelOffer, ChannelStreamEvent,
};
use objectiveai_sdk::cli::command::AgentArguments;
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
    // accepted.
    key: String,
    details: serde_json::Value,
    message: String,
    plugin_owner: Option<String>,
    plugin_name: Option<String>,
    plugin_version: Option<String>,
    agent_arguments: AgentArguments,
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

/// Why an accept-open failed.
pub enum AcceptOpenError {
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
    #[allow(clippy::too_many_arguments)]
    pub fn create_offer(
        &self,
        key: String,
        details: serde_json::Value,
        message: String,
        plugin_owner: Option<String>,
        plugin_name: Option<String>,
        plugin_version: Option<String>,
        agent_arguments: AgentArguments,
    ) -> (String, String, oneshot::Receiver<()>) {
        let channel_id = uuid::Uuid::new_v4().to_string();
        let pub_secret = uuid::Uuid::new_v4().to_string();
        let offer_frame = frame(&ChannelEvent::Offer {
            offer: ChannelOffer {
                channel_id: channel_id.clone(),
                plugin_owner: plugin_owner.clone(),
                plugin_name: plugin_name.clone(),
                plugin_version: plugin_version.clone(),
                agent_arguments: agent_arguments.clone(),
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
            plugin_owner,
            plugin_name,
            plugin_version,
            agent_arguments,
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

    /// The first-wins ACCEPT, driven by a `GET /channels/{id}` open:
    /// arbitrate the offer's oneshot → mint `S_owner` → persist the
    /// channel → unblock the publisher → withdraw the offer from every
    /// connection that saw it. Returns `S_owner`; the caller's stream
    /// delivers it as the first frame and anchors the channel's life.
    pub async fn accept_open(
        &self,
        pool: &crate::db::Pool,
        channel_id: &str,
    ) -> Result<String, AcceptOpenError> {
        let Some(offer) = self.offers.get(channel_id).map(|e| Arc::clone(e.value()))
        else {
            // No pending offer: an existing channel row means someone
            // already accepted; nothing at all means unknown/withdrawn.
            return match crate::db::channels::channel_state(pool, channel_id).await {
                Ok(Some(_)) => Err(AcceptOpenError::AlreadyAccepted),
                Ok(None) => Err(AcceptOpenError::NotFound),
                Err(e) => Err(AcceptOpenError::Db(e)),
            };
        };
        // Arbitrate: the first accept takes the oneshot. Take it out
        // under the sync lock, then do all async work lock-free.
        let winner = {
            let mut slot = offer.accept.lock().expect("accept lock");
            match slot.take() {
                Some(sender) => sender,
                None => return Err(AcceptOpenError::AlreadyAccepted),
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
                owner: offer.plugin_owner.as_deref(),
                name: offer.plugin_name.as_deref(),
                version: offer.plugin_version.as_deref(),
            },
            &offer.agent_arguments,
        )
        .await
        {
            return Err(AcceptOpenError::Db(e));
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

/// Serialize one wire event to its SSE frame string.
fn frame<T: serde::Serialize>(event: &T) -> String {
    serde_json::to_string(event).expect("wire event serializes")
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

/// RAII on an ACCEPT-mode per-channel stream: dropping it closes the
/// channel (terminal, idempotent). The DB close is async, so it runs
/// in a spawned task — the trigger's NOTIFY wakes any blocked
/// subscriber/observer.
struct ChannelCloseGuard {
    pool: crate::db::Pool,
    channel_id: String,
}

impl Drop for ChannelCloseGuard {
    fn drop(&mut self) {
        let pool = self.pool.clone();
        let channel_id = std::mem::take(&mut self.channel_id);
        tokio::spawn(async move {
            let _ = crate::db::channels::close_channel(&pool, &channel_id).await;
        });
    }
}

/// Resolve when the channel is (or becomes) closed/absent. The LISTEN
/// is attached BEFORE the state check (the `channels logs subscribe`
/// pattern), so a close landing between the check and the wait is
/// never lost. Message NOTIFYs wake the loop harmlessly — it just
/// re-checks state.
async fn closed_wait(
    pool: &crate::db::Pool,
    channel_id: &str,
) -> Result<(), crate::db::Error> {
    let mut listener = crate::db::channels::channel_event_listener(pool).await?;
    loop {
        match crate::db::channels::channel_state(pool, channel_id).await? {
            Some(crate::db::channels::ChannelState::Open) => {}
            Some(crate::db::channels::ChannelState::Closed) | None => return Ok(()),
        }
        crate::db::channels::recv_channel_event(&mut listener, channel_id).await?;
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

/// `GET /channels/{id}`: the per-channel stream. Header-auth first,
/// then mode by the `X-OBJECTIVEAI-CHANNEL-SECRET` header:
///
/// - **absent = ACCEPT-OPEN**: first-wins accept of the pending offer
///   ([`ChannelHub::accept_open`]). First frame =
///   [`ChannelStreamEvent::Secret`] (`S_owner`); the stream anchors
///   the channel — its drop closes it. Statuses: 404 unknown or
///   withdrawn id; 409 already accepted (including "channel exists,
///   no secret presented"); 500 DB.
/// - **present = OBSERVER**: the secret must match a role
///   (`role_of`, 401 otherwise; 404 unknown id). Silent until the
///   channel closes → one [`ChannelStreamEvent::Closed`] → end
///   (immediate on an already-closed channel). Observer drops close
///   nothing.
pub(crate) async fn channel_stream_handler(
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
    let secret = headers
        .get("X-OBJECTIVEAI-CHANNEL-SECRET")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    if let Some(secret) = secret {
        // OBSERVER: authenticate the channel secret, then wait out the
        // channel's life. The authoritative closed check happens
        // INSIDE closed_wait, after its LISTEN attach — a close
        // landing between channel_auth and the attach still yields an
        // immediate `closed` frame.
        let auth = match crate::db::channels::channel_auth(&pool, &id).await {
            Ok(Some(auth)) => auth,
            Ok(None) => return axum::http::StatusCode::NOT_FOUND.into_response(),
            Err(e) => {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("channel auth: {e}"),
                )
                    .into_response();
            }
        };
        if auth.role_of(&secret).is_none() {
            return axum::http::StatusCode::UNAUTHORIZED.into_response();
        }
        let stream = async_stream::stream! {
            if closed_wait(&pool, &id).await.is_ok() {
                yield Ok::<_, std::convert::Infallible>(
                    Event::default().data(frame(&ChannelStreamEvent::Closed)),
                );
            }
        };
        return Sse::new(stream).into_response();
    }

    // ACCEPT-OPEN: the open IS the accept.
    let owner_secret = match state.channels.accept_open(&pool, &id).await {
        Ok(secret) => secret,
        Err(AcceptOpenError::NotFound) => {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        }
        Err(AcceptOpenError::AlreadyAccepted) => {
            return axum::http::StatusCode::CONFLICT.into_response();
        }
        Err(AcceptOpenError::Db(e)) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("channel accept: {e}"),
            )
                .into_response();
        }
    };
    // Arm the close guard OUTSIDE the stream body and MOVE it in: an
    // async_stream body doesn't run until first poll, so a guard
    // constructed inside would never exist if the response is dropped
    // unpolled (client vanished mid-request) — the channel would leak
    // open. A moved-in guard is captured at construction; dropping the
    // unpolled stream still fires it.
    let guard = ChannelCloseGuard {
        pool: pool.clone(),
        channel_id: id.clone(),
    };
    let stream = async_stream::stream! {
        let _guard = guard;
        yield Ok::<_, std::convert::Infallible>(
            Event::default().data(frame(&ChannelStreamEvent::Secret {
                secret: owner_secret,
            })),
        );
        // Externally-driven closes (a future close/delete verb) end
        // the stream uniformly; today only this stream's own drop
        // closes the channel, so this wait normally outlives the
        // client.
        if closed_wait(&pool, &id).await.is_ok() {
            yield Ok::<_, std::convert::Infallible>(
                Event::default().data(frame(&ChannelStreamEvent::Closed)),
            );
        }
    };
    Sse::new(stream).into_response()
}
