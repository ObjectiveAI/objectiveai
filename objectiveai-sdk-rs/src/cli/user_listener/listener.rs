//! Materialized consumer of the cli daemon's `/user` endpoint — the
//! user-requests channel.
//!
//! [`UserListener`] connects once, then folds every incoming
//! [`UserEvent`] into an in-memory map of `id → UserRequest`
//! holding the CURRENTLY PENDING requests: a
//! [`Request`](UserEvent::Request) (live broadcast or connect-time
//! replay) inserts, [`Settled`](UserEvent::Settled) and
//! [`TimedOut`](UserEvent::TimedOut) remove.
//!
//! Ways to observe it:
//! - [`pending`](UserListener::pending) — async snapshot of the
//!   pending set (sorted by id).
//! - an **event callback**
//!   ([`on_event`](UserListenerBuilder::on_event)), invoked with
//!   every parsed [`UserEvent`] (settlements and timeouts included —
//!   a UI needs the edges, not just the folded set).
//! - [`subscribe`](UserListener::subscribe) — async, blocks until
//!   the next applied event.
//!
//! Replying is [`UserListener::reply`] — a plain
//! `POST /user/{id}/reply` sharing the listener's address and
//! signature; the replier identity is the caller's to provide.
//!
//! One listener = one connection: the internal pump runs until the
//! daemon socket closes; after that the view is frozen. Dropping the
//! listener aborts the pump. Reconnection is the caller's loop —
//! build a new listener.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use reqwest_eventsource::{Event, RequestBuilderExt};
use tokio::sync::{Mutex, watch};

use crate::cli::command::AgentArguments;

use super::{UserEvent, UserReply, UserReplyOutcome, UserRequest};

/// The event callback: invoked with every parsed [`UserEvent`], after
/// it is folded into the pending map.
pub type OnEvent = Box<dyn Fn(&UserEvent) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request builder rejected the URL, or opening the SSE
    /// stream failed.
    #[error("connect daemon sse: {0}")]
    Connect(#[from] reqwest_eventsource::CannotCloneRequestError),
    /// The underlying reqwest client failed to build (or a reply POST
    /// failed in transport).
    #[error("daemon sse http client: {0}")]
    Client(#[from] reqwest::Error),
    /// A reply POST returned a body that isn't a [`UserReplyOutcome`].
    #[error("user reply outcome parse: {0}")]
    OutcomeParse(#[from] serde_json::Error),
}

/// The shared inner state, held by both the listener handle and its
/// pump task.
struct Shared {
    /// `id → request` — the currently PENDING requests.
    state: Mutex<BTreeMap<String, UserRequest>>,
    /// Monotonically-bumped event counter; each applied event bumps
    /// it, waking every [`subscribe`](UserListener::subscribe) waiter.
    changes: watch::Sender<u64>,
    /// Optional push callback, invoked with every parsed event.
    on_event: Option<OnEvent>,
}

/// Unconnected configuration — [`UserListener::new`] +
/// [`UserListenerBuilder::signature`] +
/// [`UserListenerBuilder::connect`].
pub struct UserListenerBuilder {
    /// The daemon's published base address, e.g.
    /// `http://127.0.0.1:49152` — `/user` and `/user/{id}/reply` are
    /// appended.
    base_url: String,
    /// Optional auth signature, sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header.
    signature: Option<String>,
    /// Optional event callback.
    on_event: Option<OnEvent>,
}

impl UserListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` request header on the stream AND on
    /// every [`reply`](UserListener::reply). Without it the daemon
    /// must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Register a callback invoked with every parsed [`UserEvent`]
    /// after it is folded. Runs on the pump task — keep it cheap and
    /// non-blocking.
    pub fn on_event(
        mut self,
        callback: impl Fn(&UserEvent) + Send + Sync + 'static,
    ) -> Self {
        self.on_event = Some(Box::new(callback));
        self
    }

    /// Open the SSE stream and start the pump. The daemon replays
    /// every pending request first, so the view converges
    /// immediately.
    pub async fn connect(self) -> Result<UserListener, Error> {
        let url = format!("{}/user", self.base_url.trim_end_matches('/'));
        let source = connect_sse(&url, self.signature.as_deref())?;
        let shared = Arc::new(Shared {
            state: Mutex::new(BTreeMap::new()),
            changes: watch::channel(0u64).0,
            on_event: self.on_event,
        });
        let pump = tokio::spawn(pump(source, shared.clone()));
        Ok(UserListener {
            shared,
            pump,
            base_url: self.base_url,
            signature: self.signature,
        })
    }
}

/// The materialized `/user` view + reply client — see the module
/// docs. Construct via [`UserListener::new`]. Dropping it aborts the
/// background pump.
pub struct UserListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
    base_url: String,
    signature: Option<String>,
}

impl UserListener {
    /// Start building a listener from the daemon's published base
    /// address (e.g. `http://127.0.0.1:49152`).
    pub fn new(base_url: impl Into<String>) -> UserListenerBuilder {
        UserListenerBuilder {
            base_url: base_url.into(),
            signature: None,
            on_event: None,
        }
    }

    /// Snapshot the currently pending requests, sorted by id.
    pub async fn pending(&self) -> Vec<UserRequest> {
        self.shared.state.lock().await.values().cloned().collect()
    }

    /// Block until the next event is applied. Pair with
    /// [`pending`](Self::pending) in a loop, or use the
    /// [`on_event`](UserListenerBuilder::on_event) callback for
    /// guaranteed push.
    pub async fn subscribe(&self) {
        let mut rx = self.shared.changes.subscribe();
        let _ = rx.changed().await;
    }

    /// The raw change-counter receiver — for RACE-FREE condition
    /// waits: hold ONE receiver across iterations of a
    /// check-then-await loop, and an event landing between the check
    /// and the await still resolves the next `changed()` (watch
    /// receivers remember what they've seen; a one-shot
    /// [`subscribe`](Self::subscribe) can miss exactly that window).
    pub fn changes(&self) -> watch::Receiver<u64> {
        self.shared.changes.subscribe()
    }

    /// Reply to one pending request: `POST /user/{id}/reply` with the
    /// listener's signature, `identity` as the `X-OBJECTIVEAI-*`
    /// headers, and `reply` as the body. Returns the daemon's
    /// [`UserReplyOutcome`] whatever the HTTP status — only transport
    /// and parse failures error.
    pub async fn reply(
        &self,
        id: &str,
        identity: &AgentArguments,
        reply: serde_json::Value,
    ) -> Result<UserReplyOutcome, Error> {
        let url = format!(
            "{}/user/{}/reply",
            self.base_url.trim_end_matches('/'),
            id
        );
        let client = reqwest::Client::builder().build()?;
        let mut request = client.post(url).json(&UserReply { reply });
        if let Some(signature) = &self.signature {
            request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
        }
        for (header, value) in [
            ("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY", &identity.agent_instance_hierarchy),
            ("X-OBJECTIVEAI-AGENT-ID", &identity.agent_id),
            ("X-OBJECTIVEAI-AGENT-FULL-ID", &identity.agent_full_id),
            ("X-OBJECTIVEAI-AGENT-REMOTE", &identity.agent_remote),
            ("X-OBJECTIVEAI-RESPONSE-ID", &identity.response_id),
            ("X-OBJECTIVEAI-RESPONSE-IDS", &identity.response_ids),
        ] {
            if let Some(v) = value {
                request = request.header(header, v);
            }
        }
        let body = request.send().await?.text().await?;
        Ok(serde_json::from_str(&body)?)
    }
}

impl Drop for UserListener {
    fn drop(&mut self) {
        // Stop updating a view no one holds any more.
        self.pump.abort();
    }
}

/// Fold one event into the pending map: `Request` inserts,
/// `Settled`/`TimedOut` remove, `Live` (the caught-up marker) leaves
/// the state untouched — it still bumps the change counter and
/// reaches the callback like every event.
fn apply_event(state: &mut BTreeMap<String, UserRequest>, event: &UserEvent) {
    match event {
        UserEvent::Request { request } => {
            state.insert(request.id.clone(), request.clone());
        }
        UserEvent::Settled { id, .. } | UserEvent::TimedOut { id } => {
            state.remove(id);
        }
        UserEvent::Live => {}
    }
}

/// Read frames, fold each [`UserEvent`], fire the callback, bump the
/// change counter. Runs until the connection closes. Parse errors and
/// non-text frames are skipped; transport errors end the pump.
async fn pump(mut source: reqwest_eventsource::EventSource, shared: Arc<Shared>) {
    while let Some(event) = source.next().await {
        match event {
            Ok(Event::Open) => continue,
            Ok(Event::Message(message)) => {
                match serde_json::from_str::<UserEvent>(&message.data) {
                    Ok(event) => {
                        {
                            let mut state = shared.state.lock().await;
                            apply_event(&mut state, &event);
                        }
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
    let mut request = client
        .get(url)
        .header("Accept", "text/event-stream");
    if let Some(signature) = signature {
        request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
    }
    Ok(request.eventsource()?)
}
