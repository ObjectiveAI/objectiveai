//! The daemon's laboratory surface: the `/laboratory` WebSocket route
//! and the in-process laboratory registry.
//!
//! Laboratory HOSTS (`objectiveai-laboratory host` processes — one per
//! (machine, state), serving ALL of that machine's laboratories) dial
//! IN on `/laboratory`. The connection's wire order is load-bearing:
//!
//! 1. The FIRST text frame is the [`HostIdentify`] — who this HOST is:
//!    its state, its machine identity, and the FULL set of
//!    laboratories it serves. Identity always PRECEDES authorization
//!    on this endpoint. ANY state is accepted — a host is one per
//!    (machine, state) and that PAIR is its registry identity; the
//!    state scopes the HOST's containers and locks on its own
//!    machine, and a remote daemon's state name is unrelated to it.
//! 2. The SECOND frame is the standard first-message `AuthEnvelope`
//!    (verified by [`crate::websockets::daemon_auth::authenticate`],
//!    demoted to second place here).
//! 3. Then the daemon sends [`ChannelRequest`]s (stamped with the
//!    target `laboratory_id`) and the host answers [`ChannelResponse`]s
//!    correlated by id — plus uncorrelated host→daemon
//!    [`HostNotification`]s whenever the host's laboratory set changes
//!    (create/delete), which update the registry's per-host set. No
//!    scanning, no polling: the announced set + notifications ARE the
//!    daemon's laboratory knowledge.
//!
//! The set of live `/laboratory` connections IS the laboratory
//! registry: `laboratories list` snapshots it, and a host disconnect
//! removes every laboratory it served (its in-flight forwards fail
//! cleanly). There is no local-vs-remote split — machine identity is
//! the only provenance, one code path wherever the host runs. The
//! conduit and the `laboratories` commands reach laboratories by
//! calling [`LaboratoryRegistry::forward`] / [`LaboratoryRegistry::list`]
//! directly on the resident daemon's registry (via `Context`'s resident
//! hubs) — in-process, no socket.

use std::sync::Arc;

use dashmap::DashMap;
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{
    ChannelRequest, ChannelResponse, HostIdentify, HostNotification, Identify,
};
use objectiveai_sdk::machine::MachineIdentity;
use tokio::sync::{mpsc, oneshot, RwLock};

/// How long a forward waits for the host's reply. Generous — tool
/// calls and 2 MiB transfer chunks ride this; the API layer above owns
/// the real deadlines.
const FORWARD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// One connected laboratory host.
struct HostConnection {
    machine: MachineIdentity,
    /// The state this host serves ON ITS OWN MACHINE (its container
    /// names and locks are scoped to it). Half of the host's registry
    /// identity — two hosts on one machine (different states) coexist.
    state: String,
    /// The laboratories this host serves RIGHT NOW: the HostIdentify
    /// announcement, kept current by created/deleted notifications.
    labs: RwLock<IndexMap<String, Identify>>,
    /// Frames queued to the host (drained by the connection's writer
    /// half).
    tx: mpsc::UnboundedSender<ChannelRequest>,
    /// In-flight forwards awaiting the host's correlated reply.
    /// Dropped wholesale on disconnect, failing every waiter.
    pending: DashMap<String, oneshot::Sender<ChannelResponse>>,
}

/// One connected-set mutation, broadcast to the live `/laboratories/*`
/// endpoints. Payloads are RAW ids (machine or laboratory); consumers
/// rebuild from the registry rather than trusting the event's shape.
#[derive(Debug, Clone)]
pub enum LabRegistryChange {
    /// A host connection registered under this machine id (fresh
    /// connect OR a reconnect displacing its predecessor). Its whole
    /// announced laboratory set appeared with it.
    HostConnected(String),
    /// The machine id's host connection deregistered (socket closed
    /// and the entry was still its own). Every laboratory it served
    /// vanished with it.
    HostDisconnected(String),
    /// A connected host created this laboratory (a `laboratory_created`
    /// notification).
    LaboratoryCreated(String),
    /// A connected host deleted this laboratory (a `laboratory_deleted`
    /// notification).
    LaboratoryDeleted(String),
}

/// The connected-host registry, shared by the `/laboratory` route
/// (writers) and the conduit + `laboratories` commands +
/// `/laboratories/*` endpoints (readers). Keyed by `(machine id,
/// state)` — a host's full identity; hosts of ANY state register, and
/// two same-machine hosts (different states) coexist.
#[derive(Clone)]
pub struct LaboratoryRegistry {
    hosts: Arc<DashMap<(String, String), Arc<HostConnection>>>,
    /// Connected-set change feed. Send errors (no subscriber) are
    /// ignored — the feed is advisory.
    events: tokio::sync::broadcast::Sender<LabRegistryChange>,
}

impl LaboratoryRegistry {
    pub fn new() -> Self {
        Self {
            hosts: Arc::new(DashMap::new()),
            events: tokio::sync::broadcast::channel(1024).0,
        }
    }

    /// Subscribe to connected-set changes.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<LabRegistryChange> {
        self.events.subscribe()
    }

    /// Every served laboratory with the machine that serves it — the
    /// registry snapshot `laboratories list` and the `/laboratories/*`
    /// streams are built from.
    pub async fn list(&self) -> Vec<(MachineIdentity, Identify)> {
        // Clone the Arcs out first; never hold a map guard across an
        // await.
        let hosts: Vec<Arc<HostConnection>> =
            self.hosts.iter().map(|e| Arc::clone(&e)).collect();
        let mut out = Vec::new();
        for host in hosts {
            let labs = host.labs.read().await;
            out.extend(
                labs.values()
                    .map(|identify| (host.machine.clone(), identify.clone())),
            );
        }
        out
    }

    /// Whether the exact host `(machine id, state)` is connected right
    /// now.
    pub fn has_host(&self, machine_id: &str, state: &str) -> bool {
        self.hosts
            .contains_key(&(machine_id.to_string(), state.to_string()))
    }

    /// Every connected host for this machine id, as `(state, machine
    /// identity)` — the command layer applies its own preference rule
    /// over them.
    pub fn hosts_for_machine(&self, machine_id: &str) -> Vec<(String, MachineIdentity)> {
        self.hosts
            .iter()
            .filter(|e| e.key().0 == machine_id)
            .map(|e| (e.key().1.clone(), e.machine.clone()))
            .collect()
    }

    /// The identity of the exact connected host `(machine id, state)`.
    pub fn machine(&self, machine_id: &str, state: &str) -> Option<MachineIdentity> {
        self.hosts
            .get(&(machine_id.to_string(), state.to_string()))
            .map(|h| h.machine.clone())
    }

    /// The `(machine id, state)` of the host serving this laboratory,
    /// if any — derived from the per-host sets (truth), no separate
    /// index to drift. First match wins: RAW laboratory ids should be
    /// unique per machine (they are state-scoped host-side, so a
    /// cross-state duplicate is possible but degenerate).
    pub async fn host_for_laboratory(
        &self,
        laboratory_id: &str,
    ) -> Option<(String, String)> {
        let hosts: Vec<Arc<HostConnection>> =
            self.hosts.iter().map(|e| Arc::clone(&e)).collect();
        for host in hosts {
            if host.labs.read().await.contains_key(laboratory_id) {
                return Some((host.machine.id.clone(), host.state.clone()));
            }
        }
        None
    }

    /// Forward one request to the host serving `laboratory_id` and
    /// await its correlated reply. The request is stamped with the
    /// laboratory id — the host demuxes on it.
    pub async fn forward(
        &self,
        laboratory_id: &str,
        headers: indexmap::IndexMap<String, String>,
        request: objectiveai_sdk::client_objectiveai_mcp::server_request::Payload,
    ) -> Result<objectiveai_sdk::client_objectiveai_mcp::server_response::Payload, String> {
        let Some((machine_id, state)) = self.host_for_laboratory(laboratory_id).await
        else {
            return Err(format!(
                "laboratory '{laboratory_id}' is not served by any connected host"
            ));
        };
        self.forward_inner(
            &machine_id,
            &state,
            Some(laboratory_id.to_string()),
            headers,
            request,
        )
        .await
    }

    /// Forward one HOST-LEVEL request (create — no laboratory to
    /// address yet) to the exact host `(machine id, state)` and await
    /// its reply.
    pub async fn forward_to_host(
        &self,
        machine_id: &str,
        state: &str,
        headers: indexmap::IndexMap<String, String>,
        request: objectiveai_sdk::client_objectiveai_mcp::server_request::Payload,
    ) -> Result<objectiveai_sdk::client_objectiveai_mcp::server_response::Payload, String> {
        self.forward_inner(machine_id, state, None, headers, request)
            .await
    }

    async fn forward_inner(
        &self,
        machine_id: &str,
        state: &str,
        laboratory_id: Option<String>,
        headers: indexmap::IndexMap<String, String>,
        request: objectiveai_sdk::client_objectiveai_mcp::server_request::Payload,
    ) -> Result<objectiveai_sdk::client_objectiveai_mcp::server_response::Payload, String> {
        // Clone the Arc out; never hold a map guard across an await.
        let host = match self
            .hosts
            .get(&(machine_id.to_string(), state.to_string()))
        {
            Some(host) => Arc::clone(&host),
            None => {
                return Err(format!(
                    "no laboratory host connected for machine '{machine_id}' state '{state}'"
                ));
            }
        };
        let id = uuid::Uuid::new_v4().to_string();
        let (reply_tx, reply_rx) = oneshot::channel();
        host.pending.insert(id.clone(), reply_tx);
        let sent = host.tx.send(ChannelRequest {
            id: id.clone(),
            laboratory_id,
            headers,
            payload: request,
        });
        if sent.is_err() {
            host.pending.remove(&id);
            return Err(format!(
                "laboratory host for machine '{machine_id}' disconnected"
            ));
        }
        match tokio::time::timeout(FORWARD_TIMEOUT, reply_rx).await {
            Ok(Ok(response)) => Ok(response.payload),
            Ok(Err(_)) => {
                // Pending map dropped — the host disconnected.
                Err(format!(
                    "laboratory host for machine '{machine_id}' disconnected mid-request"
                ))
            }
            Err(_) => {
                host.pending.remove(&id);
                Err(format!(
                    "laboratory host for machine '{machine_id}' timed out"
                ))
            }
        }
    }
}

/// `/laboratory`: upgrade, read the HostIdentify frame, consume the
/// auth preamble (strictly second), register under the host's
/// `(machine, state)` identity, pump until disconnect.
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
                    match serde_json::from_str::<HostIdentify>(&text) {
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
        // NO state gate: the host's state scopes containers and locks
        // on ITS machine — a remote daemon's own state name is
        // unrelated. The (machine, state) pair is simply the host's
        // registry identity.
        // 2. Authorization SECOND (the standard preamble, verbatim).
        if !crate::websockets::daemon_auth::authenticate(&mut socket, state.secret.as_ref())
            .await
        {
            return;
        }
        // 3. Register under (machine id, state). A live entry means
        // either a stale duplicate (the host's `laboratories` lock
        // should prevent one) or a reconnect racing its own
        // predecessor's teardown — the NEW connection wins: displace
        // the old entry (its pending waiters fail). Same-machine hosts
        // of OTHER states are untouched.
        let host_key = (identify.machine.id.clone(), identify.state.clone());
        let (tx, mut rx) = mpsc::unbounded_channel::<ChannelRequest>();
        let labs: IndexMap<String, Identify> = identify
            .laboratories
            .into_iter()
            .map(|lab| (lab.id.clone(), lab))
            .collect();
        let host = Arc::new(HostConnection {
            machine: identify.machine,
            state: identify.state,
            labs: RwLock::new(labs),
            tx,
            pending: DashMap::new(),
        });
        state
            .laboratories
            .hosts
            .insert(host_key.clone(), Arc::clone(&host));
        let _ = state
            .laboratories
            .events
            .send(LabRegistryChange::HostConnected(host_key.0.clone()));

        // Pump: outbound requests + inbound correlated replies and
        // uncorrelated notifications.
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
                        // ChannelResponse first (it has `id`), then
                        // HostNotification (no correlation id) — the
                        // same parse strategy the API recv_loop uses.
                        if let Ok(response) = serde_json::from_str::<ChannelResponse>(&text) {
                            if let Some((_, waiter)) = host.pending.remove(&response.id) {
                                let _ = waiter.send(response);
                            }
                            continue;
                        }
                        let Ok(notification) =
                            serde_json::from_str::<HostNotification>(&text)
                        else {
                            // Forward-compat: skip frames this build
                            // doesn't know.
                            continue;
                        };
                        match notification {
                            HostNotification::LaboratoryCreated { laboratory } => {
                                let id = laboratory.id.clone();
                                host.labs.write().await.insert(id.clone(), laboratory);
                                let _ = state
                                    .laboratories
                                    .events
                                    .send(LabRegistryChange::LaboratoryCreated(id));
                            }
                            HostNotification::LaboratoryDeleted { id } => {
                                host.labs.write().await.shift_remove(&id);
                                let _ = state
                                    .laboratories
                                    .events
                                    .send(LabRegistryChange::LaboratoryDeleted(id));
                            }
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
            .hosts
            .remove_if(&host_key, |_, current| Arc::ptr_eq(current, &host));
        if removed.is_some() {
            let _ = state
                .laboratories
                .events
                .send(LabRegistryChange::HostDisconnected(host_key.0));
        }
        // Dropping `host.pending` (last Arc) fails all in-flight waiters.
    })
}
