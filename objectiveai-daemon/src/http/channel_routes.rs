//! The daemon's `/channels` endpoint — DUPLEX CHANNELS: a publisher
//! offers a channel, the first connected SSE client to ACCEPT owns it,
//! and the two exchange messages over a durable per-channel log
//! (`channels logs …`, backed by [`crate::db::channels`]).
//!
//! [`ChannelHub`] holds only the LIVE, in-memory coordination — the
//! durable channel record + message log live in Postgres. Its state:
//!
//! - **connections** — every `GET /channels` stream, keyed by a hub
//!   id, each with its own mpsc sender and a per-connection secret
//!   (`S_conn`) sent as the stream's FIRST frame. `conn_by_secret`
//!   indexes `S_conn → id` for accept lookup.
//! - **offers** — the PENDING (pre-accept) offers, keyed by channel
//!   id. An offer carries everything needed to persist the channel on
//!   accept, plus the arbitration oneshot the blocked `channels
//!   publish` command awaits and the audience set for its withdrawal.
//! - **owner_conns** — `conn_id → owned channel ids`, so dropping a
//!   connection can CLOSE every channel it owns (terminal — the
//!   publisher's next write/subscribe learns it).
//!
//! Secret flow: `S_conn` is minted per connection and sent over its
//! SSE; `S_pub` is minted at offer time and returned to the publisher's
//! command; `S_owner` is minted on accept and pushed ONLY down the
//! accepting connection's SSE (never in the accept POST response) —
//! binding the capability to the actual stream holder even if `S_conn`
//! leaks.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::response::sse::{Event, Sse};
use dashmap::DashMap;
use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::channel_listener::{
    ChannelAccept, ChannelAcceptOutcome, ChannelEvent, ChannelOffer,
};
use tokio::sync::{mpsc, oneshot};

/// One live `GET /channels` connection.
struct Connection {
    /// This connection's secret (`S_conn`).
    secret: String,
    sender: mpsc::UnboundedSender<String>,
}

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
    plugin_owner: Option<String>,
    plugin_repository: Option<String>,
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

/// The channels hub — see the module docs. Clone-shared.
#[derive(Clone)]
pub struct ChannelHub {
    connections: Arc<DashMap<u64, Connection>>,
    conn_by_secret: Arc<DashMap<String, u64>>,
    next_connection_id: Arc<AtomicU64>,
    offers: Arc<DashMap<String, Arc<Offer>>>,
    owner_conns: Arc<DashMap<u64, HashSet<String>>>,
}

impl ChannelHub {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            conn_by_secret: Arc::new(DashMap::new()),
            next_connection_id: Arc::new(AtomicU64::new(1)),
            offers: Arc::new(DashMap::new()),
            owner_conns: Arc::new(DashMap::new()),
        }
    }

    /// Register a `GET /channels` connection: allocate its id, mint its
    /// `S_conn`, send that as the FIRST frame, replay every open offer,
    /// then send [`ChannelEvent::Live`].
    fn register_connection(&self) -> (u64, mpsc::UnboundedReceiver<String>) {
        let id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        let secret = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::unbounded_channel();
        // The connection secret is ALWAYS the first frame.
        let _ = tx.send(frame(&ChannelEvent::Connection {
            secret: secret.clone(),
        }));
        self.conn_by_secret.insert(secret.clone(), id);
        self.connections.insert(id, Connection { secret, sender: tx.clone() });
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

    /// Drop a closed connection: remove it + its secret index + its
    /// ownership set, returning the channel ids it OWNED so the caller
    /// can close them in the DB. Stale ids left in offers' `offered_to`
    /// sets are harmless — their sends go nowhere.
    fn unregister_connection(&self, id: u64) -> Vec<String> {
        if let Some((_, connection)) = self.connections.remove(&id) {
            self.conn_by_secret.remove(&connection.secret);
        }
        self.owner_conns
            .remove(&id)
            .map(|(_, set)| set.into_iter().collect())
            .unwrap_or_default()
    }

    /// Create a pending offer and fan it out to every current
    /// connection. Returns `(channel_id, S_pub, accept_rx)` — the
    /// publisher's command holds `S_pub` and awaits `accept_rx`.
    #[allow(clippy::too_many_arguments)]
    pub fn create_offer(
        &self,
        key: String,
        details: serde_json::Value,
        plugin_owner: Option<String>,
        plugin_repository: Option<String>,
        plugin_version: Option<String>,
        agent_arguments: AgentArguments,
    ) -> (String, String, oneshot::Receiver<()>) {
        let channel_id = uuid::Uuid::new_v4().to_string();
        let pub_secret = uuid::Uuid::new_v4().to_string();
        let offer_frame = frame(&ChannelEvent::Offer {
            offer: ChannelOffer {
                channel_id: channel_id.clone(),
                plugin_owner: plugin_owner.clone(),
                plugin_repository: plugin_repository.clone(),
                plugin_version: plugin_version.clone(),
                agent_arguments: agent_arguments.clone(),
                key: key.clone(),
                details: details.clone(),
            },
        });
        let (accept_tx, accept_rx) = oneshot::channel();
        let offer = Arc::new(Offer {
            channel_id: channel_id.clone(),
            pub_secret: pub_secret.clone(),
            offer_frame,
            key,
            details,
            plugin_owner,
            plugin_repository,
            plugin_version,
            agent_arguments,
            accept: std::sync::Mutex::new(Some(accept_tx)),
            offered_to: std::sync::Mutex::new(HashSet::new()),
        });
        self.offers.insert(channel_id.clone(), Arc::clone(&offer));
        for connection in self.connections.iter() {
            let mut offered_to = offer.offered_to.lock().expect("offered_to lock");
            if offered_to.insert(*connection.key()) {
                let _ = connection.value().sender.send(offer.offer_frame.clone());
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

    /// Accept an offer: validate `S_conn` → arbitrate → persist the
    /// channel → push `S_owner` over the accepting connection's SSE →
    /// unblock the publisher. `Ok` carries the wire outcome (never a
    /// secret); `Err` is a DB failure (the handler 500s).
    pub async fn accept(
        &self,
        pool: &crate::db::Pool,
        conn_secret: &str,
        channel_id: &str,
    ) -> Result<ChannelAcceptOutcome, crate::db::Error> {
        let Some(conn_id) = self.conn_by_secret.get(conn_secret).map(|e| *e.value())
        else {
            return Ok(ChannelAcceptOutcome::UnknownConnection);
        };
        let Some(offer) = self.offers.get(channel_id).map(|e| Arc::clone(e.value()))
        else {
            return Ok(ChannelAcceptOutcome::NotFound);
        };
        // Arbitrate: the first accept takes the oneshot. Take it out
        // under the sync lock, then do all async work lock-free.
        let winner = {
            let mut slot = offer.accept.lock().expect("accept lock");
            match slot.take() {
                Some(sender) => sender,
                None => return Ok(ChannelAcceptOutcome::AlreadyAccepted),
            }
        };
        let owner_secret = uuid::Uuid::new_v4().to_string();
        // Persist the channel BEFORE unblocking publish — if this
        // fails, `winner` drops without firing, so publish sees the
        // offer as abandoned rather than succeeding against no row.
        crate::db::channels::insert_channel(
            pool,
            channel_id,
            &offer.pub_secret,
            &owner_secret,
            &offer.key,
            &offer.details,
            &crate::db::channels::PluginOrigin {
                owner: offer.plugin_owner.as_deref(),
                repository: offer.plugin_repository.as_deref(),
                version: offer.plugin_version.as_deref(),
            },
            &offer.agent_arguments,
        )
        .await?;
        // Record ownership so a connection drop closes this channel.
        self.owner_conns
            .entry(conn_id)
            .or_default()
            .insert(channel_id.to_string());
        // The offer is consumed.
        self.offers.remove(channel_id);
        // Push S_owner ONLY to the accepting connection's stream.
        if let Some(connection) = self.connections.get(&conn_id) {
            let _ = connection.sender.send(frame(&ChannelEvent::OwnerSecret {
                channel_id: channel_id.to_string(),
                secret: owner_secret,
            }));
        }
        // Unblock the publisher's command.
        let _ = winner.send(());
        // Everyone else who saw the offer learns it's gone.
        let mut offered_to = offer.offered_to.lock().expect("offered_to lock");
        offered_to.remove(&conn_id);
        let withdrawn = frame(&ChannelEvent::OfferWithdrawn {
            channel_id: channel_id.to_string(),
        });
        for other in offered_to.iter() {
            if let Some(connection) = self.connections.get(other) {
                let _ = connection.sender.send(withdrawn.clone());
            }
        }
        Ok(ChannelAcceptOutcome::Accepted)
    }

    /// Send one event to exactly the connections that saw `offer`.
    fn notify_offered(&self, offer: &Offer, event: &ChannelEvent) {
        let payload = frame(event);
        let offered_to = offer.offered_to.lock().expect("offered_to lock");
        for id in offered_to.iter() {
            if let Some(connection) = self.connections.get(id) {
                let _ = connection.sender.send(payload.clone());
            }
        }
    }
}

/// Serialize one [`ChannelEvent`] to its SSE frame string.
fn frame(event: &ChannelEvent) -> String {
    serde_json::to_string(event).expect("ChannelEvent serializes")
}

/// RAII: dropping the SSE stream (client gone) closes every channel
/// the connection owned. The DB close is async, so it runs in a
/// spawned task — the trigger's NOTIFY wakes any blocked subscriber.
struct ConnectionGuard {
    hub: ChannelHub,
    id: u64,
    pool: crate::db::Pool,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let owned = self.hub.unregister_connection(self.id);
        if owned.is_empty() {
            return;
        }
        let pool = self.pool.clone();
        tokio::spawn(async move {
            for channel_id in owned {
                let _ = crate::db::channels::close_channel(&pool, &channel_id).await;
            }
        });
    }
}

/// `GET /channels`: header-auth, then an SSE stream of
/// [`ChannelEvent`]s — the connection secret first, offer replay next,
/// live traffic after.
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
    // Channels need the DB (accept persists, drop closes). Resolve the
    // pool up front so the connection guard can close on drop.
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
    let hub = state.channels.clone();
    let (id, mut rx) = hub.register_connection();
    let stream = async_stream::stream! {
        let _guard = ConnectionGuard { hub, id, pool };
        while let Some(frame) = rx.recv().await {
            yield Ok::<_, std::convert::Infallible>(Event::default().data(frame));
        }
    };
    Sse::new(stream).into_response()
}

/// `POST /channels/{id}/accept`: header-auth, [`ChannelAccept`] body
/// (the caller's `S_conn`). Answers with a [`ChannelAcceptOutcome`]
/// JSON body (NO secret); the status mirrors it (200 accepted /
/// 409 already accepted / 404 unknown offer / 401 unknown connection).
pub(crate) async fn channels_accept_handler(
    axum::extract::State(state): axum::extract::State<
        crate::http::daemon_stream::DaemonHttpState,
    >,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !crate::http::daemon_auth::authenticate_header(
        &headers,
        state.global.auth_secret().as_ref(),
    ) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    let accept: ChannelAccept = match serde_json::from_slice(&body) {
        Ok(accept) => accept,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                format!("channel accept body: {e}"),
            )
                .into_response();
        }
    };
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
    let outcome = match state.channels.accept(&pool, &accept.conn_secret, &id).await {
        Ok(outcome) => outcome,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("channel accept: {e}"),
            )
                .into_response();
        }
    };
    let status = match &outcome {
        ChannelAcceptOutcome::Accepted => axum::http::StatusCode::OK,
        ChannelAcceptOutcome::AlreadyAccepted => axum::http::StatusCode::CONFLICT,
        ChannelAcceptOutcome::NotFound => axum::http::StatusCode::NOT_FOUND,
        ChannelAcceptOutcome::UnknownConnection => axum::http::StatusCode::UNAUTHORIZED,
    };
    let body = serde_json::to_string(&outcome).expect("ChannelAcceptOutcome serializes");
    (status, body).into_response()
}
