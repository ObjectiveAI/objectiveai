//! Materialized consumer of the cli daemon's `/laboratories/{id}`
//! endpoint.
//!
//! [`LaboratoriesListener`] connects once, then folds every
//! incoming [`LaboratoryInstanceEvent::Laboratory`] frame — always a
//! full-value record replace — into an in-memory
//! `Option<LaboratoryRecord>`.
//!
//! Three ways to observe it:
//! - [`laboratory`](LaboratoriesListener::laboratory) —
//!   async snapshot of the current record (`None` before the first
//!   frame).
//! - an on-change **callback**
//!   ([`on_change`](LaboratoriesListenerBuilder::on_change)),
//!   invoked with the fresh record on every frame.
//! - [`subscribe`](LaboratoriesListener::subscribe) — async,
//!   blocks until the next change.
//!
//! One listener = one connection: the internal pump runs until the
//! daemon socket closes; after that the view is frozen at its last
//! state. Dropping the listener aborts the pump. Reconnection is the
//! caller's loop — build a new listener.

use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{Mutex, watch};

use super::{LaboratoryInstanceEvent, LaboratoryRecord};

/// The on-change callback: invoked with the fresh full record after
/// each applied frame.
pub type OnChange = Box<dyn Fn(&LaboratoryRecord) + Send + Sync>;

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
    /// The latest full record — `None` until the first frame lands.
    state: Mutex<Option<LaboratoryRecord>>,
    /// A monotonically-bumped change counter. Each applied frame
    /// bumps it, waking every
    /// [`subscribe`](LaboratoriesListener::subscribe) waiter.
    changes: watch::Sender<u64>,
    /// Optional push callback, invoked with the fresh record after
    /// each frame.
    on_change: Option<OnChange>,
}

/// Unconnected configuration — [`LaboratoriesListener::new`]
/// + [`LaboratoriesListenerBuilder::signature`] +
/// [`LaboratoriesListenerBuilder::connect`].
pub struct LaboratoriesListenerBuilder {
    /// Full connect URL of the daemon's per-laboratory route, e.g.
    /// `http://127.0.0.1:49152/laboratories/my-lab`.
    url: String,
    /// Optional auth signature, sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header.
    signature: Option<String>,
    /// Optional on-change callback.
    on_change: Option<OnChange>,
}

impl LaboratoriesListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header — the daemon's SSE
    /// watcher routes authenticate by header. Without it the daemon
    /// must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Register a callback invoked with the fresh full record after
    /// every applied frame. Runs on the pump task, so keep it cheap
    /// and non-blocking; for the state on demand use
    /// [`laboratory`](LaboratoriesListener::laboratory).
    pub fn on_change(
        mut self,
        callback: impl Fn(&LaboratoryRecord) + Send + Sync + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Open the SSE stream and start the pump. The
    /// returned [`LaboratoriesListener`] immediately begins
    /// folding frames (the first is the endpoint's connect-time
    /// record).
    pub async fn connect(self) -> Result<LaboratoriesListener, Error> {
        let source = connect_sse(&self.url, self.signature.as_deref())?;

        let shared = Arc::new(Shared {
            state: Mutex::new(None),
            changes: watch::channel(0u64).0,
            on_change: self.on_change,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(LaboratoriesListener { shared, pump })
    }
}

/// The materialized `/laboratories/{id}` view — see the module docs.
/// Construct via [`LaboratoriesListener::new`]. Dropping it
/// aborts the background pump.
pub struct LaboratoriesListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl LaboratoriesListener {
    /// Start building a listener for the daemon's `/laboratories/{id}`
    /// URL (the daemon's published base address + `/laboratories/` +
    /// the raw laboratory id).
    pub fn new(url: impl Into<String>) -> LaboratoriesListenerBuilder {
        LaboratoriesListenerBuilder {
            url: url.into(),
            signature: None,
            on_change: None,
        }
    }

    /// Snapshot the current record — `None` before the first frame.
    pub async fn laboratory(&self) -> Option<LaboratoryRecord> {
        self.shared.state.lock().await.clone()
    }

    /// Block until the next frame is applied to the state. A fresh
    /// call waits for the FIRST change after it is made — pair with a
    /// [`laboratory`](Self::laboratory) read in a loop, or use the
    /// [`on_change`](LaboratoriesListenerBuilder::on_change)
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

impl Drop for LaboratoriesListener {
    fn drop(&mut self) {
        // Stop updating a view no one holds any more.
        self.pump.abort();
    }
}

/// Read frames, fold each [`LaboratoryInstanceEvent`] into
/// `shared.state`, fire the callback with the fresh record, and bump
/// the change counter. Runs until the connection closes. Parse errors
/// and non-text frames are skipped; transport errors end the pump.
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                match serde_json::from_str::<LaboratoryInstanceEvent>(&message.data) {
                    Ok(LaboratoryInstanceEvent::Laboratory { laboratory }) => {
                        {
                            let mut state = shared.state.lock().await;
                            *state = Some(laboratory.clone());
                        }
                        if let Some(callback) = &shared.on_change {
                            callback(&laboratory);
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
