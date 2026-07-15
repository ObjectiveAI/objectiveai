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
//!    (verified by [`crate::http::daemon_auth::authenticate`],
//!    demoted to second place here).
//! 3. Then the daemon sends [`ChannelRequest`]s (stamped with the
//!    target `laboratory_id`) and the host answers [`ChannelResponse`]s
//!    correlated by id — plus uncorrelated host→daemon
//!    [`HostNotification`]s whenever the host's laboratory set changes
//!    (create/delete) or a served laboratory's file tree changes (the
//!    host proxies each running lab's `/filetree` SSE verbatim), which
//!    update the registry's per-host set and per-lab materialized
//!    trees. No scanning, no polling: the announced set + notifications
//!    ARE the daemon's laboratory knowledge.
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
use objectiveai_sdk::laboratories::daemon::{
    ChannelRequest, ChannelResponse, HostIdentify, HostNotification, Identify,
};
use objectiveai_sdk::laboratories::filetree::{FileTreeEvent, FileTreeNode};
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
    /// Per-laboratory materialized file trees (the watched root's
    /// child list), folded from the host's unsolicited
    /// `laboratory_filetree` notifications: the host opens each with a
    /// synthesized snapshot on attach and streams every delta after —
    /// this map is always current, no polling. Backs the
    /// `/laboratories/{id}/filetree` SSE endpoint's connect-time
    /// snapshots.
    filetree: RwLock<IndexMap<String, Vec<FileTreeNode>>>,
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
    /// A connected host re-announced this laboratory (a
    /// `laboratory_updated` notification — its `running` state
    /// flipped).
    LaboratoryUpdated(String),
    /// A connected host deleted this laboratory (a `laboratory_deleted`
    /// notification).
    LaboratoryDeleted(String),
}

/// One live file-tree event from a served laboratory, re-broadcast to
/// the `/laboratories/{id}/filetree` SSE handlers after being folded
/// into the owning [`HostConnection`]'s materialized tree. A separate
/// feed from [`LabRegistryChange`] — file trees churn far faster than
/// the connected set, and the list/instance streams must not wake on
/// every file write.
#[derive(Debug, Clone)]
pub struct FiletreeChange {
    /// The serving host's machine id.
    pub machine_id: String,
    /// The serving host's state.
    pub state: String,
    /// The RAW laboratory id.
    pub laboratory_id: String,
    /// The event, verbatim from the host (which proxies the lab's own
    /// `/filetree` SSE) — re-emitted verbatim to SSE subscribers.
    pub event: FileTreeEvent,
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
    /// Live file-tree event feed (see [`FiletreeChange`]). A lagged
    /// subscriber resyncs from [`Self::filetree_state`] — every event
    /// is also folded there before it is sent here.
    filetree_events: tokio::sync::broadcast::Sender<FiletreeChange>,
    /// Live filetree SSE subscriber refcounts, keyed by the RESOLVED
    /// `(machine id, state, laboratory id)`. The 0→1 / 1→0 edges send
    /// `RequestPayload::Filetree { on }` to the owning host — the
    /// signal its container lifecycle runs on (`on` lazily starts the
    /// container; `off` + no MCP connections stops it). Counts belong
    /// to SUBSCRIBERS, not hosts: a host reconnect replays `on: true`
    /// for every key it owns with a live count.
    filetree_watchers: Arc<DashMap<(String, String, String), usize>>,
}

impl LaboratoryRegistry {
    pub fn new() -> Self {
        Self {
            hosts: Arc::new(DashMap::new()),
            events: tokio::sync::broadcast::channel(1024).0,
            // Capacity matches the kernel's inotify queue (and the
            // host's filetree ring): one full kernel-side burst fits
            // before a slow viewer needs its lag resync.
            filetree_events: tokio::sync::broadcast::channel(16384).0,
            filetree_watchers: Arc::new(DashMap::new()),
        }
    }

    /// Subscribe to connected-set changes.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<LabRegistryChange> {
        self.events.subscribe()
    }

    /// Subscribe to live file-tree events (all laboratories; consumers
    /// filter by id / host pin).
    pub fn filetree_subscribe(&self) -> tokio::sync::broadcast::Receiver<FiletreeChange> {
        self.filetree_events.subscribe()
    }

    /// Register one live filetree SSE subscriber for `laboratory_id`,
    /// resolved to its serving host (the `pin` when given, else the
    /// same first-match-by-id scan [`Self::forward`] uses). Bumps the
    /// per-`(machine, state, id)` count; the 0→1 edge sends
    /// `Filetree { on: true }` to that host. Returns a guard whose
    /// drop decrements (the 1→0 edge sends `off`). `None` when no
    /// connected host serves the laboratory — the subscriber still
    /// streams (empty snapshot; events flow if a host appears), it
    /// just can't drive the container lifecycle.
    pub async fn filetree_watch(
        &self,
        laboratory_id: &str,
        pin: Option<(&str, &str)>,
    ) -> Option<FiletreeWatchGuard> {
        let (machine_id, state) = match pin {
            Some((machine_id, machine_state)) => {
                (machine_id.to_string(), machine_state.to_string())
            }
            None => self.host_for_laboratory(laboratory_id).await?,
        };
        let key = (machine_id, state, laboratory_id.to_string());
        let count = {
            let mut entry = self.filetree_watchers.entry(key.clone()).or_insert(0);
            *entry += 1;
            *entry
        };
        if count == 1 {
            // AWAITED on the 0→1 edge: the host's reply lands only
            // after its lazy container start completes, so the
            // subscriber's first snapshot isn't a premature "no
            // files" against a container that's still booting.
            // Failure (host mid-restart, start error, timeout) is
            // tolerated — the stream proceeds with what exists and
            // live events flow when the container comes up.
            let _ = self.send_filetree_signal_now(key.clone(), true).await;
        }
        Some(FiletreeWatchGuard {
            registry: self.clone(),
            key,
        })
    }

    /// Fire-and-forget `Filetree { on }` to the host owning `key`.
    /// Failures (host gone, timeout) are dropped — a reconnecting
    /// host gets the current state replayed at register time, and a
    /// host that never returns treats the lab as unwatched by
    /// default.
    fn send_filetree_signal(&self, key: (String, String, String), on: bool) {
        let registry = self.clone();
        tokio::spawn(async move {
            let _ = registry.send_filetree_signal_now(key, on).await;
        });
    }

    /// The awaited form of [`Self::send_filetree_signal`] — resolves
    /// when the host replies (for `on: true`, after its lazy container
    /// start finished), bounded by the forward timeout.
    async fn send_filetree_signal_now(
        &self,
        key: (String, String, String),
        on: bool,
    ) -> Result<objectiveai_sdk::laboratories::daemon::ResponsePayload, String> {
        let (machine_id, state, laboratory_id) = key;
        self.forward_inner(
            &machine_id,
            &state,
            Some(laboratory_id),
            indexmap::IndexMap::new(),
            objectiveai_sdk::laboratories::daemon::RequestPayload::Filetree(
                objectiveai_sdk::laboratories::daemon::FiletreeRequest { on },
            ),
        )
        .await
    }

    /// The current materialized file tree (the watched root's child
    /// list) for `laboratory_id` — from the exact host when the
    /// `(machine, machine_state)` pin is given, else from the first
    /// connected host serving that id (the legacy first-match scan).
    /// Empty when nothing is known (no host, lab never started, or no
    /// snapshot yet).
    pub async fn filetree_state(
        &self,
        laboratory_id: &str,
        pin: Option<(&str, &str)>,
    ) -> Vec<FileTreeNode> {
        // Clone the Arcs out; never hold a map guard across an await.
        let hosts: Vec<Arc<HostConnection>> = match pin {
            Some((machine_id, machine_state)) => self
                .hosts
                .get(&(machine_id.to_string(), machine_state.to_string()))
                .map(|h| vec![Arc::clone(&h)])
                .unwrap_or_default(),
            None => self.hosts.iter().map(|e| Arc::clone(&e)).collect(),
        };
        for host in hosts {
            if let Some(children) = host.filetree.read().await.get(laboratory_id) {
                return children.clone();
            }
        }
        Vec::new()
    }

    /// Every served laboratory with the host serving it — machine
    /// identity + the host's STATE (laboratory ids are only unique
    /// per (machine, state)) — the registry snapshot `laboratories
    /// list` and the `/laboratories/*` streams are built from.
    pub async fn list(&self) -> Vec<(MachineIdentity, String, Identify)> {
        // Clone the Arcs out first; never hold a map guard across an
        // await.
        let hosts: Vec<Arc<HostConnection>> =
            self.hosts.iter().map(|e| Arc::clone(&e)).collect();
        let mut out = Vec::new();
        for host in hosts {
            let labs = host.labs.read().await;
            out.extend(labs.values().map(|identify| {
                (host.machine.clone(), host.state.clone(), identify.clone())
            }));
        }
        out
    }

    /// Whether the exact host `(machine id, state)` is connected right
    /// now.
    pub fn has_host(&self, machine_id: &str, state: &str) -> bool {
        self.hosts
            .contains_key(&(machine_id.to_string(), state.to_string()))
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
    /// laboratory id — the host demuxes on it. When the caller knows
    /// the exact host (`machine` + `machine_state`, e.g. from an
    /// attachment row via the McpKind), routing is DIRECT; an absent
    /// pair falls back to the legacy first-match-by-id scan.
    pub async fn forward(
        &self,
        laboratory_id: &str,
        machine: Option<&str>,
        machine_state: Option<&str>,
        headers: indexmap::IndexMap<String, String>,
        request: objectiveai_sdk::laboratories::daemon::RequestPayload,
    ) -> Result<objectiveai_sdk::laboratories::daemon::ResponsePayload, String> {
        let (machine_id, state) = match (machine, machine_state) {
            (Some(machine), Some(machine_state)) => {
                (machine.to_string(), machine_state.to_string())
            }
            _ => match self.host_for_laboratory(laboratory_id).await {
                Some(host) => host,
                None => {
                    return Err(format!(
                        "laboratory '{laboratory_id}' is not served by any connected host"
                    ));
                }
            },
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
        request: objectiveai_sdk::laboratories::daemon::RequestPayload,
    ) -> Result<objectiveai_sdk::laboratories::daemon::ResponsePayload, String> {
        self.forward_inner(machine_id, state, None, headers, request)
            .await
    }

    async fn forward_inner(
        &self,
        machine_id: &str,
        state: &str,
        laboratory_id: Option<String>,
        headers: indexmap::IndexMap<String, String>,
        request: objectiveai_sdk::laboratories::daemon::RequestPayload,
    ) -> Result<objectiveai_sdk::laboratories::daemon::ResponsePayload, String> {
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
        // Transfer-family ops are timeout-free: an archive can exceed
        // any fixed cap, and the host disconnect (pending-map drop) is
        // the failure signal. Everything else keeps the standard cap.
        let timeout_free = {
            use objectiveai_sdk::laboratories::daemon::RequestPayload as P;
            matches!(
                request,
                P::ExportBegin(_)
                    | P::ExportRead(_)
                    | P::ExportAbort(_)
                    | P::ImportBegin(_)
                    | P::ImportWrite(_)
                    | P::ImportEnd(_)
                    | P::ImportAbort(_)
                    | P::LocalTransfer(_)
            )
        };
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
        if timeout_free {
            return match reply_rx.await {
                Ok(response) => Ok(response.payload),
                // Pending map dropped — the host disconnected.
                Err(_) => Err(format!(
                    "laboratory host for machine '{machine_id}' disconnected mid-request"
                )),
            };
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

/// RAII registration of one filetree SSE subscriber — see
/// [`LaboratoryRegistry::filetree_watch`]. Dropping it decrements the
/// count; the last drop sends `Filetree { on: false }` to the host.
pub struct FiletreeWatchGuard {
    registry: LaboratoryRegistry,
    key: (String, String, String),
}

impl Drop for FiletreeWatchGuard {
    fn drop(&mut self) {
        let mut last = false;
        self.registry
            .filetree_watchers
            .remove_if_mut(&self.key, |_, count| {
                *count -= 1;
                last = *count == 0;
                last
            });
        if last {
            self.registry
                .send_filetree_signal(self.key.clone(), false);
        }
    }
}

/// `/laboratory`: upgrade, read the HostIdentify frame, consume the
/// auth preamble (strictly second), register under the host's
/// `(machine, state)` identity, pump until disconnect.
pub(crate) async fn laboratory_handler(
    axum::extract::State(state): axum::extract::State<
        crate::http::daemon_stream::DaemonHttpState,
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
        if !crate::http::daemon_auth::authenticate(&mut socket, state.secret.as_ref())
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
            filetree: RwLock::new(IndexMap::new()),
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
        // Replay the filetree-watch state: a (re)connecting host
        // defaults every lab to unwatched, so every key it owns with
        // live subscribers gets a fresh `on: true` — without this, a
        // daemon-side watcher that outlives a host restart would
        // never restart the container.
        for entry in state.laboratories.filetree_watchers.iter() {
            let (machine_id, host_state, laboratory_id) = entry.key();
            if *machine_id == host_key.0 && *host_state == host_key.1 && *entry.value() > 0 {
                state.laboratories.send_filetree_signal(
                    (machine_id.clone(), host_state.clone(), laboratory_id.clone()),
                    true,
                );
            }
        }

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
                                // A recreate under live subscribers needs a
                                // fresh `on` edge — the host cleared its
                                // watch demand at delete, but our count
                                // never dropped, so no subscriber edge will
                                // fire.
                                let key = (
                                    host_key.0.clone(),
                                    host_key.1.clone(),
                                    id.clone(),
                                );
                                if state
                                    .laboratories
                                    .filetree_watchers
                                    .get(&key)
                                    .is_some_and(|count| *count > 0)
                                {
                                    state
                                        .laboratories
                                        .send_filetree_signal(key, true);
                                }
                                let _ = state
                                    .laboratories
                                    .events
                                    .send(LabRegistryChange::LaboratoryCreated(id));
                            }
                            HostNotification::LaboratoryUpdated { laboratory } => {
                                let id = laboratory.id.clone();
                                host.labs.write().await.insert(id.clone(), laboratory);
                                let _ = state
                                    .laboratories
                                    .events
                                    .send(LabRegistryChange::LaboratoryUpdated(id));
                            }
                            HostNotification::LaboratoryDeleted { id } => {
                                host.labs.write().await.shift_remove(&id);
                                host.filetree.write().await.shift_remove(&id);
                                let _ = state
                                    .laboratories
                                    .events
                                    .send(LabRegistryChange::LaboratoryDeleted(id));
                            }
                            HostNotification::LaboratoryFiletree { id, event } => {
                                // The host's control lane (Deleted) and
                                // filetree ring are separate lanes: a
                                // `laboratory_deleted` can overtake a
                                // stale ring delta, and folding that
                                // straggler would resurrect a phantom
                                // tree via `or_default`. A LIVE lab is
                                // always announced (HostIdentify /
                                // Created) before its first filetree
                                // frame, so unknown-lab events are
                                // exactly the stragglers — drop them.
                                if !host.labs.read().await.contains_key(&id) {
                                    continue;
                                }
                                // Fold FIRST, then feed subscribers — a
                                // lagged subscriber resyncs from the
                                // folded state, so this order means it
                                // never misses what it skipped.
                                {
                                    let mut filetree = host.filetree.write().await;
                                    event
                                        .clone()
                                        .apply(filetree.entry(id.clone()).or_default());
                                }
                                let _ = state.laboratories.filetree_events.send(
                                    FiletreeChange {
                                        machine_id: host.machine.id.clone(),
                                        state: host.state.clone(),
                                        laboratory_id: id,
                                        event,
                                    },
                                );
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
