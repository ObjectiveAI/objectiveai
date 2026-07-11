//! Materialized consumer of the cli daemon's `/laboratories/list`
//! endpoint.
//!
//! [`WebSocketLaboratoriesListListener`] is NOT a raw event stream —
//! it connects once, then folds every incoming [`LaboratoryEvent`]
//! into an in-memory, self-updating map of `id → LaboratoryStatus`: a
//! [`Snapshot`](LaboratoryEvent::Snapshot) replaces the whole set,
//! [`Upserted`](LaboratoryEvent::Upserted) replaces one laboratory by
//! id (introducing it if unseen), and
//! [`Removed`](LaboratoryEvent::Removed) drops one. Per-lab attachment
//! detail is `/laboratories/{id}`'s job.
//!
//! Three ways to observe it:
//! - [`laboratories`](WebSocketLaboratoriesListListener::laboratories)
//!   — async snapshot of the current set (sorted by id).
//! - an on-change **callback**
//!   ([`on_change`](WebSocketLaboratoriesListListenerBuilder::on_change)),
//!   invoked with the full refreshed set on every applied change.
//! - [`subscribe`](WebSocketLaboratoriesListListener::subscribe) —
//!   async, blocks until the next change.
//!
//! One listener = one connection: the internal pump runs until the
//! daemon socket closes; after that the view is frozen at its last
//! state. Dropping the listener aborts the pump. Reconnection (the
//! daemon's address changes across restarts) is the caller's loop —
//! build a new listener.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tokio::sync::{Mutex, watch};
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::{LaboratoryEvent, LaboratoryStatus};
use crate::cli::command::command_executor::websocket::AuthEnvelope;

/// The on-change callback: invoked with the full current laboratory
/// set (sorted by id) after each applied [`LaboratoryEvent`].
pub type OnChange = Box<dyn Fn(&[LaboratoryStatus]) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The URL failed to build into a client upgrade request, or the
    /// connection/upgrade itself failed.
    #[error("connect daemon laboratories websocket: {0}")]
    Connect(tungstenite::Error),
    /// Sending the auth preamble on the freshly-opened connection failed.
    #[error("daemon laboratories websocket: {0}")]
    Ws(tungstenite::Error),
}

/// The shared inner state, held by both the listener handle and its
/// pump task.
struct Shared {
    /// `id → status`. A `BTreeMap` so iteration (snapshots, the
    /// callback) is sorted by id.
    state: Mutex<BTreeMap<String, LaboratoryStatus>>,
    /// A monotonically-bumped change counter. Each applied event bumps
    /// it, waking every
    /// [`subscribe`](WebSocketLaboratoriesListListener::subscribe)
    /// waiter.
    changes: watch::Sender<u64>,
    /// Optional push callback, invoked with the full set after each
    /// change.
    on_change: Option<OnChange>,
}

impl Shared {
    fn statuses(state: &BTreeMap<String, LaboratoryStatus>) -> Vec<LaboratoryStatus> {
        state.values().cloned().collect()
    }
}

/// Unconnected configuration —
/// [`WebSocketLaboratoriesListListener::new`] +
/// [`WebSocketLaboratoriesListListenerBuilder::signature`] +
/// [`WebSocketLaboratoriesListListenerBuilder::connect`].
pub struct WebSocketLaboratoriesListListenerBuilder {
    /// Full connect URL of the daemon's laboratories route, e.g.
    /// `ws://127.0.0.1:49152/laboratories/list`.
    url: String,
    /// Optional auth signature, sent in the [`AuthEnvelope`] preamble
    /// right after connecting.
    signature: Option<String>,
    /// Optional on-change callback.
    on_change: Option<OnChange>,
}

impl WebSocketLaboratoriesListListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent verbatim in the
    /// [`AuthEnvelope`] preamble — the connection's first text frame,
    /// the same shape every daemon route expects. Without it the
    /// daemon must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Register a callback invoked with the full current laboratory
    /// set (sorted by id) after every applied change. Runs on the
    /// pump task, so keep it cheap and non-blocking; for the full
    /// state on demand use
    /// [`laboratories`](WebSocketLaboratoriesListListener::laboratories).
    pub fn on_change(
        mut self,
        callback: impl Fn(&[LaboratoryStatus]) + Send + Sync + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Upgrade, send the auth preamble, and start the pump. The
    /// returned [`WebSocketLaboratoriesListListener`] immediately
    /// begins folding events (the first is the endpoint's
    /// connect-time snapshot).
    pub async fn connect(self) -> Result<WebSocketLaboratoriesListListener, Error> {
        let upgrade = self
            .url
            .as_str()
            .into_client_request()
            .map_err(Error::Connect)?;
        let (mut ws, _response) = tokio_tungstenite::connect_async(upgrade)
            .await
            .map_err(Error::Connect)?;

        // The auth preamble — always the connection's first text
        // frame, `{"signature": null}` against a secretless daemon.
        let auth = serde_json::to_string(&AuthEnvelope {
            signature: self.signature,
        })
        .expect("AuthEnvelope serialization is infallible");
        ws.send(tungstenite::Message::Text(auth.into()))
            .await
            .map_err(Error::Ws)?;

        let shared = Arc::new(Shared {
            state: Mutex::new(BTreeMap::new()),
            changes: watch::channel(0u64).0,
            on_change: self.on_change,
        });
        let pump = tokio::spawn(pump(ws, shared.clone()));
        Ok(WebSocketLaboratoriesListListener { shared, pump })
    }
}

/// The materialized `/laboratories/list` view — see the module docs.
/// Construct via [`WebSocketLaboratoriesListListener::new`]. Dropping
/// it aborts the background pump.
pub struct WebSocketLaboratoriesListListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl WebSocketLaboratoriesListListener {
    /// Start building a listener for the daemon's `/laboratories/list`
    /// URL (the daemon's published base address + `/laboratories/list`).
    pub fn new(url: impl Into<String>) -> WebSocketLaboratoriesListListenerBuilder {
        WebSocketLaboratoriesListListenerBuilder {
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
    /// [`on_change`](WebSocketLaboratoriesListListenerBuilder::on_change)
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

impl Drop for WebSocketLaboratoriesListListener {
    fn drop(&mut self) {
        // Stop updating a view no one holds any more.
        self.pump.abort();
    }
}

/// Fold one event into the map. `Snapshot` replaces; `Upserted`
/// replaces one laboratory by id (introducing it if unseen);
/// `Removed` drops one.
fn apply_event(state: &mut BTreeMap<String, LaboratoryStatus>, event: LaboratoryEvent) {
    match event {
        LaboratoryEvent::Snapshot { laboratories } => {
            state.clear();
            for laboratory in laboratories {
                state.insert(laboratory.id.clone(), laboratory);
            }
        }
        LaboratoryEvent::Upserted { laboratory } => {
            state.insert(laboratory.id.clone(), laboratory);
        }
        LaboratoryEvent::Removed { id } => {
            state.remove(&id);
        }
    }
}

/// Read frames, fold each `LaboratoryEvent` into `shared.state`, fire
/// the callback with the refreshed set, and bump the change counter.
/// Runs until the connection closes. Parse errors and non-text frames
/// are skipped; transport errors end the pump.
async fn pump(
    mut ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    shared: Arc<Shared>,
) {
    loop {
        match ws.next().await {
            Some(Ok(tungstenite::Message::Text(text))) => {
                match serde_json::from_str::<LaboratoryEvent>(&text) {
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
            // Control / non-text frames: tungstenite answers pings itself.
            Some(Ok(tungstenite::Message::Close(_))) | None => break,
            Some(Ok(_)) => continue,
            Some(Err(_)) => break,
        }
    }
}
