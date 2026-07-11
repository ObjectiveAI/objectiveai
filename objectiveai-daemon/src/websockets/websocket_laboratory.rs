//! The daemon's laboratory surface: the `/laboratory` WebSocket route
//! and the in-process laboratory registry.
//!
//! Laboratory MANAGERS (`objectiveai-laboratory` processes — local or
//! remote, the daemon cannot tell and does not care) dial IN on
//! `/laboratory`. The connection's wire order is load-bearing:
//!
//! 1. The FIRST text frame is the [`Identify`] — who this laboratory
//!    is. Identity always PRECEDES authorization on this endpoint.
//! 2. The SECOND frame is the standard first-message `AuthEnvelope`
//!    (verified by [`crate::websockets::daemon_auth::authenticate`],
//!    demoted to second place here).
//! 3. Then the daemon sends [`ChannelRequest`]s and the manager
//!    answers [`ChannelResponse`]s, correlated by id.
//!
//! The set of live `/laboratory` connections IS the laboratory
//! registry: `laboratories list` snapshots it, and a disconnect removes
//! the laboratory (its in-flight forwards fail cleanly). The conduit and
//! the `laboratories` commands reach connected laboratories by calling
//! [`LaboratoryRegistry::forward`] / [`LaboratoryRegistry::list`]
//! directly on the resident daemon's registry (via `Context`'s resident
//! hubs) — in-process, no socket.

use std::sync::Arc;

use dashmap::DashMap;
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{
    ChannelRequest, ChannelResponse, Identify,
};
use tokio::sync::{mpsc, oneshot};

/// How long a forward waits for the manager's reply. Generous — tool
/// calls and 2 MiB transfer chunks ride this; the API layer above owns
/// the real deadlines.
const FORWARD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// One connected laboratory manager.
struct LabConnection {
    identify: Identify,
    /// Frames queued to the manager (drained by the connection's
    /// writer half).
    tx: mpsc::UnboundedSender<ChannelRequest>,
    /// In-flight forwards awaiting the manager's correlated reply.
    /// Dropped wholesale on disconnect, failing every waiter.
    pending: DashMap<String, oneshot::Sender<ChannelResponse>>,
}

/// One connected-set mutation, broadcast to the live `/laboratories/*`
/// endpoints. Payload is the RAW laboratory id; consumers rebuild
/// from the registry rather than trusting the event's shape.
#[derive(Debug, Clone)]
pub enum LabRegistryChange {
    /// A manager connection registered under this id (fresh connect
    /// OR a reconnect displacing its predecessor).
    Connected(String),
    /// The id's manager connection deregistered (socket closed and
    /// the entry was still its own).
    Disconnected(String),
}

/// The connected-laboratory registry, shared by the `/laboratory`
/// route (writers) and the socket + `laboratories list` +
/// `/laboratories/*` endpoints (readers).
#[derive(Clone)]
pub struct LaboratoryRegistry {
    labs: Arc<DashMap<String, Arc<LabConnection>>>,
    /// Connected-set change feed. Send errors (no subscriber) are
    /// ignored — the feed is advisory.
    events: tokio::sync::broadcast::Sender<LabRegistryChange>,
}

impl LaboratoryRegistry {
    pub fn new() -> Self {
        Self {
            labs: Arc::new(DashMap::new()),
            events: tokio::sync::broadcast::channel(1024).0,
        }
    }

    /// Subscribe to connected-set changes.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<LabRegistryChange> {
        self.events.subscribe()
    }

    /// Identity snapshots of every connected laboratory.
    pub fn list(&self) -> Vec<Identify> {
        self.labs.iter().map(|e| e.identify.clone()).collect()
    }

    /// Forward one request to a connected laboratory and await its
    /// correlated reply.
    pub async fn forward(
        &self,
        laboratory_id: &str,
        headers: indexmap::IndexMap<String, String>,
        request: objectiveai_sdk::client_objectiveai_mcp::server_request::Payload,
    ) -> Result<objectiveai_sdk::client_objectiveai_mcp::server_response::Payload, String> {
        // Clone the Arc out; never hold a map guard across an await.
        let lab = match self.labs.get(laboratory_id) {
            Some(lab) => Arc::clone(&lab),
            None => return Err(format!("laboratory '{laboratory_id}' is not connected")),
        };
        let id = uuid::Uuid::new_v4().to_string();
        let (reply_tx, reply_rx) = oneshot::channel();
        lab.pending.insert(id.clone(), reply_tx);
        let sent = lab.tx.send(ChannelRequest { id: id.clone(), headers, payload: request });
        if sent.is_err() {
            lab.pending.remove(&id);
            return Err(format!("laboratory '{laboratory_id}' disconnected"));
        }
        match tokio::time::timeout(FORWARD_TIMEOUT, reply_rx).await {
            Ok(Ok(response)) => Ok(response.payload),
            Ok(Err(_)) => {
                // Pending map dropped — the manager disconnected.
                Err(format!("laboratory '{laboratory_id}' disconnected mid-request"))
            }
            Err(_) => {
                lab.pending.remove(&id);
                Err(format!("laboratory '{laboratory_id}' timed out"))
            }
        }
    }
}

/// `/laboratory`: upgrade, read the Identify frame, consume the auth
/// preamble (strictly second), register, pump until disconnect.
pub(crate) async fn laboratory_handler(
    axum::extract::State(state): axum::extract::State<
        crate::websockets::daemon_stream::DaemonWsState,
    >,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        // 1. Identity FIRST. Control frames are skipped like the auth
        // reader does; anything unparseable closes the connection.
        let identify = loop {
            match socket.recv().await {
                Some(Ok(axum::extract::ws::Message::Text(text))) => {
                    match serde_json::from_str::<Identify>(&text) {
                        Ok(identify) => break identify,
                        Err(_) => {
                            let _ = socket.send(axum::extract::ws::Message::Close(None)).await;
                            return;
                        }
                    }
                }
                Some(Ok(axum::extract::ws::Message::Close(_))) | Some(Err(_)) | None => return,
                Some(Ok(_)) => continue,
            }
        };
        // 2. Authorization SECOND (the standard preamble, verbatim).
        if !crate::websockets::daemon_auth::authenticate(&mut socket, state.secret.as_ref())
            .await
        {
            return;
        }
        // 3. Register. A live entry under this id means either a stale
        // duplicate (the id lock should prevent one) or a reconnect
        // racing its own predecessor's teardown — the NEW connection
        // wins: displace the old entry (its pending waiters fail).
        let (tx, mut rx) = mpsc::unbounded_channel::<ChannelRequest>();
        let lab = Arc::new(LabConnection {
            identify: identify.clone(),
            tx,
            pending: DashMap::new(),
        });
        state.laboratories.labs.insert(identify.id.clone(), Arc::clone(&lab));
        let _ = state
            .laboratories
            .events
            .send(LabRegistryChange::Connected(identify.id.clone()));

        // Pump: outbound requests + inbound correlated replies.
        loop {
            tokio::select! {
                queued = rx.recv() => match queued {
                    Some(request) => {
                        let Ok(frame) = serde_json::to_string(&request) else {
                            continue;
                        };
                        if socket
                            .send(axum::extract::ws::Message::Text(frame.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    // Registry entry displaced (reconnect race) — this
                    // connection is dead weight; close it out.
                    None => break,
                },
                received = socket.recv() => match received {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        let Ok(response) = serde_json::from_str::<ChannelResponse>(&text) else {
                            continue;
                        };
                        if let Some((_, waiter)) = lab.pending.remove(&response.id) {
                            let _ = waiter.send(response);
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => continue,
                },
            }
        }

        // Deregister — but only if the entry is still OURS (a reconnect
        // may have displaced it already).
        let removed = state
            .laboratories
            .labs
            .remove_if(&identify.id, |_, current| Arc::ptr_eq(current, &lab));
        if removed.is_some() {
            let _ = state
                .laboratories
                .events
                .send(LabRegistryChange::Disconnected(identify.id.clone()));
        }
        // Dropping `lab.pending` (last Arc) fails all in-flight waiters.
    })
}
