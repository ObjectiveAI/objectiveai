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
//! - an **event callback**
//!   ([`on_event`](ChannelListenerBuilder::on_event)), invoked with
//!   every parsed [`ChannelEvent`].
//! - [`subscribe`](ChannelListener::subscribe) — async, blocks until
//!   the next applied event.
//!
//! Accepting is [`ChannelListener::accept`]: a bare
//! `POST /channels/{id}/accept` (first-wins) whose `200` body carries
//! the owner secret (`S_owner`) — the per-channel capability for
//! `channels logs reply|list|open|subscribe` and `channels close`.
//! The daemon tracks no liveness; a channel stays open until someone
//! runs `channels close` with either of its secrets.
//!
//! One listener = one connection: the internal pump runs until the
//! daemon socket closes; after that the view is frozen. Dropping the
//! listener aborts the pump. Reconnection is the caller's loop — build
//! a new listener.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{Mutex, watch};

use super::{ChannelAccepted, ChannelEvent, ChannelOffer};

/// The event callback: invoked with every parsed [`ChannelEvent`],
/// after it is folded.
pub type OnEvent = Box<dyn Fn(&ChannelEvent) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request builder rejected the URL, or opening the SSE
    /// stream failed.
    #[error("connect daemon sse: {0}")]
    Connect(#[from] reqwest_eventsource::CannotCloneRequestError),
    /// The underlying reqwest client failed to build or the request
    /// failed in transport.
    #[error("daemon sse http client: {0}")]
    Client(#[from] reqwest::Error),
    /// No pending offer / no channel with that id (404) — unknown or
    /// already withdrawn.
    #[error("channel not found")]
    NotFound,
    /// The offer was already accepted by someone else (409).
    #[error("channel already accepted")]
    AlreadyAccepted,
    /// The daemon refused the credentials (401).
    #[error("channel accept unauthorized")]
    Unauthorized,
    /// Any other non-success status on the accept.
    #[error("channel accept status: {0}")]
    Status(reqwest::StatusCode),
    /// The accept's `200` body wasn't a
    /// [`ChannelAccepted`](super::ChannelAccepted).
    #[error("channel accept body parse: {0}")]
    AcceptedParse(#[from] serde_json::Error),
}

/// The shared inner state, held by both the listener handle and its
/// pump task.
struct Shared {
    /// `channel_id → offer` — the currently OPEN offers.
    offers: Mutex<BTreeMap<String, ChannelOffer>>,
    /// Monotonically-bumped event counter; each applied event bumps
    /// it, waking every [`subscribe`](ChannelListener::subscribe)
    /// waiter.
    changes: watch::Sender<u64>,
    /// Optional push callback, invoked with every parsed event.
    on_event: Option<OnEvent>,
}

/// Unconnected configuration — [`ChannelListener::new`] +
/// [`ChannelListenerBuilder::signature`] +
/// [`ChannelListenerBuilder::connect`].
pub struct ChannelListenerBuilder {
    /// The daemon's published base address, e.g.
    /// `http://127.0.0.1:49152` — `/channels` and `/channels/{id}`
    /// are appended.
    base_url: String,
    /// Optional auth signature, sent as the `X-OBJECTIVEAI-SIGNATURE`
    /// request header.
    signature: Option<String>,
    /// Optional event callback.
    on_event: Option<OnEvent>,
}

impl ChannelListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header on the stream AND on
    /// every [`accept`](ChannelListener::accept). Without it the
    /// daemon must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Register a callback invoked with every parsed [`ChannelEvent`]
    /// after it is folded. Runs on the pump task — keep it cheap and
    /// non-blocking.
    pub fn on_event(
        mut self,
        callback: impl Fn(&ChannelEvent) + Send + Sync + 'static,
    ) -> Self {
        self.on_event = Some(Box::new(callback));
        self
    }

    /// Open the SSE stream and start the pump. The daemon replays
    /// every open offer first, so the view converges immediately.
    pub async fn connect(self) -> Result<ChannelListener, Error> {
        let url = format!("{}/channels", self.base_url.trim_end_matches('/'));
        let source = connect_sse(&url, self.signature.as_deref())?;
        let shared = Arc::new(Shared {
            offers: Mutex::new(BTreeMap::new()),
            changes: watch::channel(0u64).0,
            on_event: self.on_event,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(ChannelListener {
            shared,
            pump,
            base_url: self.base_url,
            signature: self.signature,
        })
    }
}

/// The materialized `/channels` offer view + accept client — see the
/// module docs. Construct via [`ChannelListener::new`]. Dropping it
/// aborts the background pump.
pub struct ChannelListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
    base_url: String,
    signature: Option<String>,
}

impl ChannelListener {
    /// Start building a listener from the daemon's published base
    /// address (e.g. `http://127.0.0.1:49152`).
    pub fn new(base_url: impl Into<String>) -> ChannelListenerBuilder {
        ChannelListenerBuilder {
            base_url: base_url.into(),
            signature: None,
            on_event: None,
        }
    }

    /// Snapshot the currently open offers, sorted by channel id.
    pub async fn pending(&self) -> Vec<ChannelOffer> {
        self.shared.offers.lock().await.values().cloned().collect()
    }

    /// Block until the next event is applied. Pair with
    /// [`pending`](Self::pending) in a loop, or use the
    /// [`on_event`](ChannelListenerBuilder::on_event) callback for
    /// guaranteed push.
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

    /// Accept an open offer: a bare `POST /channels/{id}/accept`
    /// (first-wins). Returns the owner secret (`S_owner`) from the
    /// response body — the per-channel capability for
    /// `channels logs reply|list|open|subscribe` and `channels close`.
    pub async fn accept(&self, channel_id: &str) -> Result<String, Error> {
        let url = format!(
            "{}/channels/{}/accept",
            self.base_url.trim_end_matches('/'),
            channel_id
        );
        let client = reqwest::Client::builder().build()?;
        let mut request = client.post(url);
        if let Some(signature) = &self.signature {
            request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
        }
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(match status {
                reqwest::StatusCode::NOT_FOUND => Error::NotFound,
                reqwest::StatusCode::CONFLICT => Error::AlreadyAccepted,
                reqwest::StatusCode::UNAUTHORIZED => Error::Unauthorized,
                _ => Error::Status(status),
            });
        }
        let body = response.text().await?;
        let accepted: ChannelAccepted = serde_json::from_str(&body)?;
        Ok(accepted.secret)
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

/// Read frames, fold each [`ChannelEvent`], fire the callback, bump
/// the change counter. Runs until the connection closes. Parse errors
/// and non-text frames are skipped; transport errors end the pump.
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                match serde_json::from_str::<ChannelEvent>(&message.data) {
                    Ok(event) => {
                        apply_event(&shared, &event).await;
                        if let Some(callback) = &shared.on_event {
                            callback(&event);
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

/// Open the daemon's SSE stream: request `text/event-stream`, stamp
/// `X-OBJECTIVEAI-SIGNATURE` when a signature is present.
fn connect_sse(
    url: &str,
    signature: Option<&str>,
) -> Result<reqwest_eventsource::EventSource, Error> {
    let client = reqwest::Client::builder().build()?;
    let mut request = client.get(url).header("Accept", "text/event-stream");
    if let Some(signature) = signature {
        request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
    }
    Ok(request.eventsource()?)
}
