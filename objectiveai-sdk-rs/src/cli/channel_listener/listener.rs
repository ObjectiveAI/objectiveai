//! Materialized consumer of the cli daemon's `/channels` endpoint —
//! the duplex-channels surface.
//!
//! [`ChannelListener`] connects once, then folds every incoming
//! [`ChannelEvent`]:
//! - [`Connection`](ChannelEvent::Connection) captures this
//!   connection's secret (`S_conn`), needed to accept offers.
//! - [`Offer`](ChannelEvent::Offer) inserts into the pending-offers
//!   map; [`OfferWithdrawn`](ChannelEvent::OfferWithdrawn) and
//!   [`Closed`](ChannelEvent::Closed) remove.
//! - [`OwnerSecret`](ChannelEvent::OwnerSecret) records the channel's
//!   owner secret (`S_owner`) and fulfils any pending
//!   [`accept`](ChannelListener::accept).
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
//! Accepting is [`ChannelListener::accept`]: it POSTs the stored
//! `S_conn` to `/channels/{id}/accept`, then awaits the owner secret
//! the daemon pushes back over the SSE — so the caller must be the
//! process holding this connection.
//!
//! One listener = one connection: the internal pump runs until the
//! daemon socket closes; after that the view is frozen. Dropping the
//! listener aborts the pump. Reconnection is the caller's loop — build
//! a new listener.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{oneshot, Mutex, watch};

use super::{ChannelAccept, ChannelAcceptOutcome, ChannelEvent, ChannelOffer};

/// The event callback: invoked with every parsed [`ChannelEvent`],
/// after it is folded.
pub type OnEvent = Box<dyn Fn(&ChannelEvent) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request builder rejected the URL, or opening the SSE
    /// stream failed.
    #[error("connect daemon sse: {0}")]
    Connect(#[from] reqwest_eventsource::CannotCloneRequestError),
    /// The underlying reqwest client failed to build (or an accept
    /// POST failed in transport).
    #[error("daemon sse http client: {0}")]
    Client(#[from] reqwest::Error),
    /// An accept POST returned a body that isn't a
    /// [`ChannelAcceptOutcome`].
    #[error("channel accept outcome parse: {0}")]
    OutcomeParse(#[from] serde_json::Error),
    /// [`accept`](ChannelListener::accept) was called before the
    /// connection secret arrived (no [`ChannelEvent::Connection`]
    /// frame yet).
    #[error("channel listener not connected: no connection secret yet")]
    NotConnected,
    /// The daemon rejected the accept (not [`ChannelAcceptOutcome::Accepted`]).
    #[error("channel accept refused: {0:?}")]
    Accept(ChannelAcceptOutcome),
    /// The accept was accepted but the pump closed before the owner
    /// secret was delivered.
    #[error("channel listener pump closed before owner secret")]
    PumpClosed,
}

/// The shared inner state, held by both the listener handle and its
/// pump task.
struct Shared {
    /// This connection's secret (`S_conn`), from the first
    /// [`ChannelEvent::Connection`] frame.
    conn_secret: Mutex<Option<String>>,
    /// `channel_id → offer` — the currently OPEN offers.
    offers: Mutex<BTreeMap<String, ChannelOffer>>,
    /// `channel_id → S_owner` — owner secrets for channels this
    /// connection accepted.
    owner_secrets: Mutex<BTreeMap<String, String>>,
    /// In-flight [`accept`](ChannelListener::accept) calls awaiting
    /// their owner secret, keyed by channel id.
    accept_waiters: Mutex<HashMap<String, oneshot::Sender<String>>>,
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
    /// `http://127.0.0.1:49152` — `/channels` and
    /// `/channels/{id}/accept` are appended.
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

    /// Open the SSE stream and start the pump. The daemon sends the
    /// connection secret first, then replays every open offer, so the
    /// view converges immediately.
    pub async fn connect(self) -> Result<ChannelListener, Error> {
        let url = format!("{}/channels", self.base_url.trim_end_matches('/'));
        let source = connect_sse(&url, self.signature.as_deref())?;
        let shared = Arc::new(Shared {
            conn_secret: Mutex::new(None),
            offers: Mutex::new(BTreeMap::new()),
            owner_secrets: Mutex::new(BTreeMap::new()),
            accept_waiters: Mutex::new(HashMap::new()),
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

/// The materialized `/channels` view + accept client — see the module
/// docs. Construct via [`ChannelListener::new`]. Dropping it aborts
/// the background pump.
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

    /// This connection's secret (`S_conn`), once the first
    /// [`ChannelEvent::Connection`] frame has arrived.
    pub async fn connection_secret(&self) -> Option<String> {
        self.shared.conn_secret.lock().await.clone()
    }

    /// Snapshot the currently open offers, sorted by channel id.
    pub async fn pending(&self) -> Vec<ChannelOffer> {
        self.shared.offers.lock().await.values().cloned().collect()
    }

    /// The owner secret (`S_owner`) for a channel this connection has
    /// accepted, if known.
    pub async fn owner_secret(&self, channel_id: &str) -> Option<String> {
        self.shared.owner_secrets.lock().await.get(channel_id).cloned()
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

    /// Accept an open offer: `POST /channels/{id}/accept` with this
    /// connection's `S_conn`, then await the owner secret (`S_owner`)
    /// the daemon pushes back over the SSE. Returns `S_owner` — the
    /// per-channel capability for `channels logs reply|list|open|
    /// subscribe`.
    ///
    /// Errors if the connection secret hasn't arrived yet
    /// ([`Error::NotConnected`]), the daemon refuses the accept
    /// ([`Error::Accept`]), or the pump closes before the secret is
    /// delivered ([`Error::PumpClosed`]).
    pub async fn accept(&self, channel_id: &str) -> Result<String, Error> {
        let conn_secret = self
            .shared
            .conn_secret
            .lock()
            .await
            .clone()
            .ok_or(Error::NotConnected)?;
        // Register the waiter BEFORE the POST so a fast OwnerSecret
        // frame (the daemon pushes it the moment it processes the
        // accept) is never missed.
        let (tx, rx) = oneshot::channel();
        self.shared
            .accept_waiters
            .lock()
            .await
            .insert(channel_id.to_string(), tx);

        let url = format!(
            "{}/channels/{}/accept",
            self.base_url.trim_end_matches('/'),
            channel_id
        );
        let client = reqwest::Client::builder().build()?;
        let mut request = client.post(url).json(&ChannelAccept { conn_secret });
        if let Some(signature) = &self.signature {
            request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
        }
        let body = request.send().await?.text().await?;
        let outcome: ChannelAcceptOutcome = serde_json::from_str(&body)?;
        if !matches!(outcome, ChannelAcceptOutcome::Accepted) {
            self.shared.accept_waiters.lock().await.remove(channel_id);
            return Err(Error::Accept(outcome));
        }
        // The secret may already have landed over the SSE between the
        // POST completing and here — take it directly if so.
        if let Some(secret) =
            self.shared.owner_secrets.lock().await.get(channel_id).cloned()
        {
            self.shared.accept_waiters.lock().await.remove(channel_id);
            return Ok(secret);
        }
        rx.await.map_err(|_| Error::PumpClosed)
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
        ChannelEvent::Connection { secret } => {
            *shared.conn_secret.lock().await = Some(secret.clone());
        }
        ChannelEvent::Offer { offer } => {
            shared
                .offers
                .lock()
                .await
                .insert(offer.channel_id.clone(), offer.clone());
        }
        ChannelEvent::OfferWithdrawn { channel_id }
        | ChannelEvent::Closed { channel_id } => {
            shared.offers.lock().await.remove(channel_id);
        }
        ChannelEvent::OwnerSecret { channel_id, secret } => {
            shared
                .owner_secrets
                .lock()
                .await
                .insert(channel_id.clone(), secret.clone());
            if let Some(waiter) =
                shared.accept_waiters.lock().await.remove(channel_id)
            {
                let _ = waiter.send(secret.clone());
            }
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
