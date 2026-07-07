//! Materialized consumer of the cli daemon's `/agents` endpoint.
//!
//! [`WebSocketAgentsListener`] is NOT a raw event stream — it connects
//! once, then folds every incoming [`AgentEvent`] into an in-memory,
//! self-updating view of the current agent set (keyed by
//! `agent_instance_hierarchy`): a [`Snapshot`](AgentEvent::Snapshot)
//! replaces the whole set, [`Activated`](AgentEvent::Activated) /
//! [`Updated`](AgentEvent::Updated) upsert one agent, and
//! [`Deactivated`](AgentEvent::Deactivated) flips one to inactive (and
//! stamps its `last_active_at`). It keeps the record on deactivation — the
//! view mirrors the endpoint's "all agents" semantics.
//!
//! Three ways to observe it:
//! - [`agents`](WebSocketAgentsListener::agents) — async snapshot of the
//!   current set (sorted by AIH).
//! - an on-change **callback** ([`on_change`](WebSocketAgentsListenerBuilder::on_change)),
//!   invoked with the full refreshed set on every applied change.
//! - [`subscribe`](WebSocketAgentsListener::subscribe) — async, blocks
//!   until the next change.
//!
//! One listener = one connection: the internal pump runs until the daemon
//! socket closes; after that the view is frozen at its last state.
//! Dropping the listener aborts the pump. Reconnection (the daemon's
//! address changes across restarts) is the caller's loop — build a new
//! listener.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tokio::sync::{Mutex, watch};
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::{AgentEvent, AgentRecord};
use crate::cli::command::command_executor::websocket::AuthEnvelope;

/// The on-change callback: invoked with the full current agent set (sorted
/// by AIH) after each applied [`AgentEvent`].
pub type OnChange = Box<dyn Fn(&[AgentRecord]) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The URL failed to build into a client upgrade request, or the
    /// connection/upgrade itself failed.
    #[error("connect daemon agents websocket: {0}")]
    Connect(tungstenite::Error),
    /// Sending the auth preamble on the freshly-opened connection failed.
    #[error("daemon agents websocket: {0}")]
    Ws(tungstenite::Error),
}

/// The shared inner state, held by both the listener handle and its pump
/// task.
struct Shared {
    /// The current agent set, keyed by `agent_instance_hierarchy`. A
    /// `BTreeMap` so iteration (snapshots, the callback) is sorted by AIH.
    state: Mutex<BTreeMap<String, AgentRecord>>,
    /// A monotonically-bumped change counter. Each applied event bumps it,
    /// waking every [`subscribe`](WebSocketAgentsListener::subscribe)
    /// waiter.
    changes: watch::Sender<u64>,
    /// Optional push callback, invoked with the full set after each change.
    on_change: Option<OnChange>,
}

/// Unconnected configuration — [`WebSocketAgentsListener::new`] +
/// [`WebSocketAgentsListenerBuilder::signature`] +
/// [`WebSocketAgentsListenerBuilder::on_change`] +
/// [`WebSocketAgentsListenerBuilder::connect`].
pub struct WebSocketAgentsListenerBuilder {
    /// Full connect URL of the daemon's agents route, e.g.
    /// `ws://127.0.0.1:49152/agents`.
    url: String,
    /// Optional auth signature, sent in the [`AuthEnvelope`] preamble right
    /// after connecting.
    signature: Option<String>,
    /// Optional on-change callback.
    on_change: Option<OnChange>,
}

impl WebSocketAgentsListenerBuilder {
    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`), sent verbatim in the
    /// [`AuthEnvelope`] preamble — the connection's first text frame.
    /// Without it the daemon must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Register a callback invoked with the full current agent set (sorted
    /// by AIH) after every applied change. Runs on the pump task, so keep
    /// it cheap and non-blocking; for the full state on demand use
    /// [`agents`](WebSocketAgentsListener::agents).
    pub fn on_change(
        mut self,
        callback: impl Fn(&[AgentRecord]) + Send + Sync + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Upgrade, send the auth preamble, and start the pump. The returned
    /// [`WebSocketAgentsListener`] immediately begins folding events into
    /// its state (the first is the endpoint's connect-time snapshot).
    pub async fn connect(self) -> Result<WebSocketAgentsListener, Error> {
        let upgrade = self
            .url
            .as_str()
            .into_client_request()
            .map_err(Error::Connect)?;
        let (mut ws, _response) = tokio_tungstenite::connect_async(upgrade)
            .await
            .map_err(Error::Connect)?;

        // The auth preamble — always the connection's first text frame,
        // `{"signature": null}` against a secretless daemon.
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
        Ok(WebSocketAgentsListener { shared, pump })
    }
}

/// The materialized `/agents` view — see the module docs. Construct via
/// [`WebSocketAgentsListener::new`]. Dropping it aborts the background
/// pump.
pub struct WebSocketAgentsListener {
    shared: Arc<Shared>,
    pump: tokio::task::JoinHandle<()>,
}

impl WebSocketAgentsListener {
    /// Start building a listener for the daemon's `/agents` URL (the
    /// daemon's published base address + `/agents`).
    pub fn new(url: impl Into<String>) -> WebSocketAgentsListenerBuilder {
        WebSocketAgentsListenerBuilder {
            url: url.into(),
            signature: None,
            on_change: None,
        }
    }

    /// Snapshot the current agent set, sorted by `agent_instance_hierarchy`.
    pub async fn agents(&self) -> Vec<AgentRecord> {
        self.shared.state.lock().await.values().cloned().collect()
    }

    /// Block until the next change is applied to the state. A fresh call
    /// waits for the FIRST change after it is made, so a change that lands
    /// between a preceding [`agents`](Self::agents) read and this call is
    /// not observed by it — pair with the read in a loop, or use the
    /// [`on_change`](WebSocketAgentsListenerBuilder::on_change) callback for
    /// guaranteed push.
    pub async fn subscribe(&self) {
        // A receiver from `subscribe` is caught up to the current version,
        // so `changed` resolves on the next bump. `Err` only if the sender
        // dropped (pump gone) — treat as "no more changes" and return.
        let mut rx = self.shared.changes.subscribe();
        let _ = rx.changed().await;
    }
}

impl Drop for WebSocketAgentsListener {
    fn drop(&mut self) {
        // Stop updating a view no one holds any more.
        self.pump.abort();
    }
}

/// Fold one event into the current set. `Snapshot` replaces; `Activated` /
/// `Updated` upsert; `Deactivated` flips one to inactive in place (kept, to
/// mirror the endpoint's all-agents view).
fn apply_event(state: &mut BTreeMap<String, AgentRecord>, event: AgentEvent) {
    match event {
        AgentEvent::Snapshot { agents } => {
            state.clear();
            for agent in agents {
                state.insert(agent.agent_instance_hierarchy.clone(), agent);
            }
        }
        AgentEvent::Activated { agent } | AgentEvent::Updated { agent } => {
            state.insert(agent.agent_instance_hierarchy.clone(), agent);
        }
        AgentEvent::Deactivated {
            agent_instance_hierarchy,
            last_active_at,
        } => {
            if let Some(record) = state.get_mut(&agent_instance_hierarchy) {
                record.active = false;
                record.last_active_at = last_active_at;
            }
        }
    }
}

/// Read frames, fold each `AgentEvent` into `shared.state`, fire the
/// callback with the refreshed set, and bump the change counter. Runs until
/// the connection closes. Parse errors and non-text frames are skipped;
/// transport errors end the pump.
async fn pump(
    mut ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    shared: Arc<Shared>,
) {
    while let Some(message) = ws.next().await {
        let text = match message {
            Ok(tungstenite::Message::Text(text)) => text,
            // Control / non-text frames: tungstenite answers pings itself.
            Ok(tungstenite::Message::Close(_)) | Err(_) => break,
            Ok(_) => continue,
        };
        let Ok(event) = serde_json::from_str::<AgentEvent>(&text) else {
            // Skip a frame we can't parse rather than tearing down.
            continue;
        };
        // Apply under the lock; clone the refreshed set only if a callback
        // needs it, and release the lock before invoking it.
        let snapshot = {
            let mut state = shared.state.lock().await;
            apply_event(&mut state, event);
            shared
                .on_change
                .as_ref()
                .map(|_| state.values().cloned().collect::<Vec<_>>())
        };
        if let (Some(callback), Some(snapshot)) = (&shared.on_change, &snapshot) {
            callback(snapshot);
        }
        shared.changes.send_modify(|version| {
            *version = version.wrapping_add(1);
        });
    }
}
