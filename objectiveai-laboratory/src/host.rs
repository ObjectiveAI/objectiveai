//! The host's request server — the machine-wide layer above
//! [`LabServer`].
//!
//! One [`HostServer`] serves EVERY laboratory in this (machine, state),
//! shared by all daemon channels. It does three jobs:
//!
//! - **Demux**: laboratory-scoped [`ChannelRequest`]s route by their
//!   stamped `laboratory_id` to a per-laboratory [`LabServer`], created
//!   lazily on the first op (podman `start` + published-port probe —
//!   containers run on demand, and everything the host started is
//!   stopped again at shutdown).
//! - **Host-level ops**: `LaboratoryCreate` / `LaboratoryDelete` run
//!   podman in-process (the same module the subcommands use) — the
//!   container lives on whatever machine the daemon forwarded to.
//! - **Notification fan-out**: every create/delete broadcasts a
//!   [`HostNotification`] to EVERY connected daemon, so all views stay
//!   current without scanning. Channel attach (identify snapshot +
//!   subscription) is atomic against these broadcasts — a change
//!   concurrent with a handshake is never lost (see `attach_lock`).
//!
//! It also OWNS the container lifecycle: a laboratory wants to run
//! while it has ≥1 live MCP connection ([`LabServer::has_connections`])
//! OR ≥1 daemon-side filetree watcher ([`RequestPayload::Filetree`],
//! per channel, default off). Either demand lazily starts the
//! container; when BOTH are gone the container is stopped after a
//! short grace ([`STOP_GRACE`], re-checked — never removed: its
//! filesystem survives for the next lazy start). A daemon channel
//! disconnecting withdraws all of ITS demand (its filetree watches and
//! its MCP sessions), so a dead daemon can never pin a container.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use objectiveai_sdk::laboratories::daemon::{
    ChannelRequest, ChannelResponse, CreateRequest, HostIdentify, HostNotification,
    Identify, IdentifyMount, JsonRpcResult, LocalTransferRequest, LocalTransferResult,
    RequestPayload, ResponsePayload, TransferAck,
};

use objectiveai_sdk::laboratories::filetree::{FileTreeEvent, FileTreeNode};
use objectiveai_sdk::machine::MachineIdentity;

use crate::podman;
use crate::server::LabServer;
/// How long a laboratory stays up after its LAST demand (MCP
/// connection or filetree watcher) disappears, before the idle stop —
/// re-checked at expiry, so any new demand in the window cancels it.
/// Long enough that back-to-back agent completions (each opening and
/// dropping a session) don't thrash podman start/stop.
const STOP_GRACE: std::time::Duration = std::time::Duration::from_secs(30);

/// The machine-wide server: lazy per-laboratory [`LabServer`]s plus
/// the outbound senders of every live daemon channel (the notification
/// fan-out targets).
pub struct HostServer {
    podman: podman::Podman,
    state: String,
    /// `<objectiveai_dir>/bin` — where the injected
    /// `objectiveai-mcp-laboratory` binary lives (create needs it).
    bin_dir: PathBuf,
    machine: MachineIdentity,
    /// Per-laboratory servers. The cell initializes ONCE on the first
    /// routed op (podman `start` + port probe); a failed init leaves it
    /// empty so the next op retries. An INITIALIZED cell marks a
    /// container this host started — [`Self::stop_started`] stops
    /// exactly those.
    labs: DashMap<String, Arc<tokio::sync::OnceCell<Arc<LabServer>>>>,
    /// One outbound frame sender per connected daemon channel, keyed by
    /// a host-minted registration id. Dead senders are dropped on the
    /// next broadcast.
    outbound: DashMap<u64, tokio::sync::mpsc::UnboundedSender<String>>,
    next_outbound: AtomicU64,
    /// Per-laboratory materialized file trees (the watched root — the
    /// lab's cwd — child list), folded from each running lab's
    /// `/filetree` SSE by its pump. Read by [`Self::attach_channel`] to
    /// open late daemons with a synthesized snapshot; both sides hold
    /// `attach_lock`, so snapshot vs. delta ordering holds by
    /// construction.
    filetree: DashMap<String, Vec<FileTreeNode>>,
    /// The per-laboratory filetree pump tasks — aborted on delete and
    /// on idle stop (and dying with the process on shutdown).
    filetree_pumps: DashMap<String, tokio::task::JoinHandle<()>>,
    /// Per-laboratory filetree DEMAND: the daemon channels that
    /// currently hold ≥1 filetree subscriber for the lab
    /// ([`RequestPayload::Filetree`], edge-triggered per channel,
    /// default off). Half of the wants-to-run condition; the other
    /// half is the lab's live MCP connections.
    filetree_watchers: DashMap<String, std::collections::HashSet<u64>>,
    /// Per-laboratory start/stop serialization: held across the lazy
    /// container init AND the idle stop, so an op racing a stop waits
    /// and re-starts cleanly instead of hitting a half-stopped
    /// container.
    lifecycle: DashMap<String, Arc<tokio::sync::Mutex<()>>>,
    /// Serializes [`Self::attach_channel`] (snapshot + subscribe)
    /// against [`Self::broadcast`], closing the classic race: a
    /// create/delete concurrent with a channel handshake either lands
    /// in that channel's HostIdentify list, in a notification it
    /// receives, or both — never in neither. Duplicates are idempotent
    /// upserts daemon-side; a miss would be a stale view until the
    /// next reconnect.
    attach_lock: tokio::sync::Mutex<()>,
}

impl HostServer {
    pub fn new(bin_dir: PathBuf, state: String, machine: MachineIdentity) -> Self {
        Self {
            podman: podman::Podman::new(bin_dir.clone()),
            state,
            bin_dir,
            machine,
            labs: DashMap::new(),
            outbound: DashMap::new(),
            next_outbound: AtomicU64::new(0),
            filetree: DashMap::new(),
            filetree_pumps: DashMap::new(),
            filetree_watchers: DashMap::new(),
            lifecycle: DashMap::new(),
            attach_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Attach a freshly-connected daemon channel: enqueue its
    /// [`HostIdentify`] (built from podman's CURRENT laboratory set —
    /// podman is the only source of truth, nothing mirrored to drift)
    /// and the auth envelope as the channel's first two frames, then
    /// register its sender for notification fan-out — all under
    /// `attach_lock`, atomically against broadcasts (see the field
    /// docs). Everything rides the ONE `reply_tx` writer, so wire
    /// order (identify, auth, then replies/notifications) holds by
    /// construction. Returns the id to [`Self::detach_channel`] with.
    pub async fn attach_channel(
        &self,
        reply_tx: tokio::sync::mpsc::UnboundedSender<String>,
        auth_frame: String,
    ) -> u64 {
        let _guard = self.attach_lock.lock().await;
        let identify = HostIdentify {
            state: self.state.clone(),
            machine: self.machine.clone(),
            laboratories: self.identify_all().await,
        };
        let identify_frame =
            serde_json::to_string(&identify).expect("identify serializes");
        let _ = reply_tx.send(identify_frame);
        let _ = reply_tx.send(auth_frame);
        // A synthesized file-tree snapshot per WATCHED laboratory, so
        // this daemon's materialized trees start current — the same
        // snapshot-then-deltas contract the lab endpoint itself opens
        // with. Under `attach_lock`, atomic against the pumps' folds.
        for entry in self.filetree.iter() {
            let notification = HostNotification::LaboratoryFiletree {
                id: entry.key().clone(),
                event: FileTreeEvent::Snapshot {
                    children: entry.value().clone(),
                },
            };
            if let Ok(frame) = serde_json::to_string(&notification) {
                let _ = reply_tx.send(frame);
            }
        }
        let id = self.next_outbound.fetch_add(1, Ordering::Relaxed);
        self.outbound.insert(id, reply_tx);
        id
    }

    /// One event off a laboratory's `/filetree` SSE (called by that
    /// lab's pump): fold it into the materialized tree, then fan it out
    /// to every connected daemon — one `attach_lock` hold for both, so
    /// an attaching channel either sees the fold in its synthesized
    /// snapshot or receives the broadcast, never neither.
    pub(crate) async fn filetree_event(&self, id: &str, event: FileTreeEvent) {
        let Ok(frame) = serde_json::to_string(&HostNotification::LaboratoryFiletree {
            id: id.to_string(),
            event: event.clone(),
        }) else {
            return;
        };
        let _guard = self.attach_lock.lock().await;
        event.apply(self.filetree.entry(id.to_string()).or_default().value_mut());
        self.outbound.retain(|_, tx| tx.send(frame.clone()).is_ok());
    }

    /// Detach a disconnected daemon channel: drop its notification
    /// sender and withdraw ALL of its demand — its filetree watches
    /// and its MCP sessions — then schedule the idle check for every
    /// laboratory it touched. A dead daemon never pins a container.
    pub fn detach_channel(self: &Arc<Self>, id: u64) {
        self.outbound.remove(&id);
        let mut affected: Vec<String> = Vec::new();
        self.filetree_watchers.retain(|lab_id, channels| {
            if channels.remove(&id) {
                affected.push(lab_id.clone());
            }
            !channels.is_empty()
        });
        for entry in self.labs.iter() {
            if let Some(server) = entry.value().get() {
                server.drop_channel(id);
                if !server.has_connections() {
                    affected.push(entry.key().clone());
                }
            }
        }
        affected.sort();
        affected.dedup();
        for lab_id in affected {
            self.schedule_stop_check(&lab_id);
        }
    }

    /// The CURRENT laboratory set, straight from podman. A read
    /// failure identifies an empty set (the daemon still gets
    /// create/delete notifications later).
    async fn identify_all(&self) -> Vec<Identify> {
        match podman::laboratory::list(&self.podman, &self.state).await {
            Ok(labs) => labs.into_iter().map(crate::identify_from_info).collect(),
            Err(e) => {
                eprintln!("list laboratories: {e}");
                Vec::new()
            }
        }
    }

    /// Serve one request from `channel`; the reply echoes the
    /// correlation id. Host-level ops (create/delete/local-transfer)
    /// and the lifecycle-owning filetree watch state run here;
    /// everything else demuxes by `laboratory_id` to a lazily-started
    /// [`LabServer`].
    pub async fn handle(self: &Arc<Self>, channel: u64, request: ChannelRequest) -> ChannelResponse {
        match &request.payload {
            RequestPayload::Create(req) => {
                let result = self.create_laboratory(req).await;
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::Create(result),
                };
            }
            RequestPayload::Delete(req) => {
                let result = self.delete_laboratory(&req.id).await;
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::Delete(result),
                };
            }
            // Host-level like create/delete — it addresses TWO
            // laboratories on THIS host, so it never demuxes to one
            // LabServer.
            RequestPayload::LocalTransfer(req) => {
                let result = self.local_transfer(req).await;
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::LocalTransfer(result),
                };
            }
            // Lab-scoped but LIFECYCLE-owning, so it never demuxes (a
            // watch must not fail just because a start is in flight).
            RequestPayload::Filetree(req) => {
                let Some(lab_id) = request.laboratory_id.clone() else {
                    return ChannelResponse {
                        payload: reject(
                            &request.payload,
                            -32600,
                            "missing laboratory_id on a laboratory-scoped request".into(),
                        ),
                        id: request.id,
                    };
                };
                let result = self.set_filetree_watch(channel, &lab_id, req.on).await;
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::Filetree(result),
                };
            }
            _ => {}
        }
        let Some(lab_id) = request.laboratory_id.clone() else {
            return ChannelResponse {
                payload: reject(
                    &request.payload,
                    -32600,
                    "missing laboratory_id on a laboratory-scoped request".into(),
                ),
                id: request.id,
            };
        };
        // A session-ending op may leave the lab idle — schedule the
        // graced stop check after serving it.
        let ends_session = matches!(
            request.payload,
            RequestPayload::SessionTerminate | RequestPayload::Drop(_)
        );
        let response = match self.lab_server(&lab_id).await {
            Ok(server) => server.handle(channel, request).await,
            Err(message) => ChannelResponse {
                payload: reject(&request.payload, -32603, message),
                id: request.id,
            },
        };
        if ends_session {
            self.schedule_stop_check(&lab_id);
        }
        response
    }

    /// `RequestPayload::Filetree`: set this channel's filetree demand
    /// for the laboratory. `on` lazily STARTS the container (the watch
    /// is the demand — the daemon holds ≥1 live subscriber); `off`
    /// withdraws it and schedules the idle check.
    async fn set_filetree_watch(
        self: &Arc<Self>,
        channel: u64,
        lab_id: &str,
        on: bool,
    ) -> JsonRpcResult<TransferAck> {
        if on {
            self.filetree_watchers
                .entry(lab_id.to_string())
                .or_default()
                .insert(channel);
            if let Err(message) = self.lab_server(lab_id).await {
                return rpc_err(-32603, message);
            }
        } else {
            self.filetree_watchers
                .remove_if_mut(lab_id, |_, channels| {
                    channels.remove(&channel);
                    channels.is_empty()
                });
            self.schedule_stop_check(lab_id);
        }
        JsonRpcResult::Ok {
            result: TransferAck {},
        }
    }

    /// The per-laboratory start/stop lock.
    fn lifecycle(&self, id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.lifecycle
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Schedule the graced idle check: after [`STOP_GRACE`], stop the
    /// container if it STILL has no demand. Spurious wakeups are
    /// harmless (the check re-verifies everything under the lifecycle
    /// lock), so callers fire this on every possibly-idle edge.
    fn schedule_stop_check(self: &Arc<Self>, id: &str) {
        let host = Arc::clone(self);
        let id = id.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(STOP_GRACE).await;
            host.stop_if_idle(&id).await;
        });
    }

    /// Stop the laboratory's container if it is running with no demand
    /// — no filetree watchers AND no live MCP connections. Serialized
    /// against the lazy start by the lifecycle lock; stopped, never
    /// removed (the filesystem survives; the materialized filetree
    /// keeps its last state — the view freezes until the next start
    /// re-snapshots).
    async fn stop_if_idle(self: &Arc<Self>, id: &str) {
        let lock = self.lifecycle(id);
        let _guard = lock.lock().await;
        // Started at all?
        let Some(server) = self.labs.get(id).and_then(|cell| cell.get().cloned()) else {
            return;
        };
        if self
            .filetree_watchers
            .get(id)
            .is_some_and(|channels| !channels.is_empty())
            || server.has_connections()
            || server.has_transfers()
        {
            return;
        }
        if let Some((_, pump)) = self.filetree_pumps.remove(id) {
            pump.abort();
        }
        if let Err(e) = podman::laboratory::stop(&self.podman, &self.state, id).await {
            eprintln!("idle-stop laboratory '{id}': {e}");
        }
        // Uninitialize: the next op (or watch) lazily starts again.
        self.labs.remove(id);
        self.broadcast_updated(id).await;
    }

    /// The laboratory's server, starting its container on first use.
    /// A successful start also spawns the lab's filetree pump (the
    /// container's `/filetree` SSE proxied to every daemon). The init
    /// runs under the lifecycle lock, so a start racing an idle stop
    /// waits for the stop to finish and brings the container back up
    /// cleanly.
    async fn lab_server(self: &Arc<Self>, id: &str) -> Result<Arc<LabServer>, String> {
        // ALWAYS under the lifecycle lock — deliberately no
        // check-the-cell fast path: a lock-free read could hand out a
        // LabServer whose container an in-flight idle stop is tearing
        // down, silently satisfying new demand with a dying container
        // (a `Filetree { on }` racing the stop would reply Ok and
        // nothing would ever restart). Waiting out the stop means the
        // cell is gone by the time we look, and the demand re-starts
        // cleanly. Uncontended, the lock is a few atomic ops.
        let lock = self.lifecycle(id);
        let _guard = lock.lock().await;
        // Re-fetch under the lock: an idle stop may have removed (or a
        // concurrent start created) the cell while we waited.
        let cell = self
            .labs
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::OnceCell::new()))
            .clone();
        // A fresh start (vs. the already-initialized fast case) gets
        // re-announced below so list subscribers see `running` flip.
        let fresh_start = cell.get().is_none();
        let server = cell.get_or_try_init(|| async {
            // Start-not-create: a stopped container resumes with its
            // filesystem intact.
            podman::laboratory::start(&self.podman, &self.state, id)
                .await
                .map_err(|e| format!("start laboratory '{id}': {e}"))?;
            match podman::laboratory::host_port(&self.podman, &self.state, id).await {
                Ok(port) => {
                    let base_url = format!("http://127.0.0.1:{port}");
                    self.spawn_filetree_pump(id, &base_url).await;
                    Ok(Arc::new(LabServer::new(base_url)))
                }
                Err(e) => {
                    // We just started it — don't leak a running
                    // container behind a failed init.
                    let _ = podman::laboratory::stop(&self.podman, &self.state, id).await;
                    Err(format!("laboratory '{id}' port: {e}"))
                }
            }
        })
        .await
        .cloned();
        if fresh_start && server.is_ok() {
            self.broadcast_updated(id).await;
        }
        server
    }

    /// Start the laboratory's filetree pump, watching its cwd (the
    /// workspace; podman's record is the source of truth — an empty or
    /// unreadable cwd falls back to the endpoint's default). Replaces —
    /// and aborts — any predecessor for the id (delete + recreate).
    async fn spawn_filetree_pump(self: &Arc<Self>, id: &str, base_url: &str) {
        let path = match podman::laboratory::list(&self.podman, &self.state).await {
            Ok(labs) => labs
                .into_iter()
                .find(|l| l.id == id)
                .map(|l| l.cwd)
                .filter(|cwd| !cwd.is_empty()),
            Err(_) => None,
        };
        let handle = tokio::spawn(crate::filetree::pump(
            Arc::clone(self),
            id.to_string(),
            base_url.to_string(),
            path,
        ));
        if let Some(old) = self.filetree_pumps.insert(id.to_string(), handle) {
            old.abort();
        }
    }

    /// `LaboratoryCreate`: podman create + MCP binary injection
    /// (container NOT started), echo the created spec, broadcast
    /// `laboratory_created` to every connected daemon.
    async fn create_laboratory(
        &self,
        req: &CreateRequest,
    ) -> JsonRpcResult<Identify> {
        // Authoritative image validation: any daemon can send this
        // host anything, so the split reference is checked HERE (the
        // CLI validates earlier only for friendlier errors). Fully
        // qualified by construction — podman never short-name-resolves.
        if let Err(message) = req.image.validate() {
            return rpc_err(-32602, format!("image: {message}"));
        }
        // Reserved-prefix ⇔ agent provenance, BIDIRECTIONALLY: an
        // `oai-agent-` id must carry its agent_full_id (only the CLI
        // conduit's on-the-fly create does), and agent provenance must
        // live under the reserved prefix. User creates pass neither —
        // a manual create squatting on the namespace is rejected here
        // authoritatively, whatever daemon sent it.
        let agent_prefixed = req
            .id
            .starts_with(objectiveai_sdk::agent::AGENT_LABORATORY_ID_PREFIX);
        match (agent_prefixed, req.agent_full_id.is_some()) {
            (true, false) => {
                return rpc_err(
                    -32602,
                    format!(
                        "laboratory id '{}' uses the reserved agent-laboratory prefix '{}' but carries no agent_full_id",
                        req.id,
                        objectiveai_sdk::agent::AGENT_LABORATORY_ID_PREFIX,
                    ),
                );
            }
            (false, true) => {
                return rpc_err(
                    -32602,
                    format!(
                        "laboratory '{}' carries agent_full_id but its id is not under the reserved '{}' prefix",
                        req.id,
                        objectiveai_sdk::agent::AGENT_LABORATORY_ID_PREFIX,
                    ),
                );
            }
            _ => {}
        }
        // Ids are one URL path segment on the daemons'
        // `/laboratories/{id}` routes — reject `/` here
        // authoritatively, whatever daemon sent the create.
        if req.id.contains('/') {
            return rpc_err(
                -32602,
                format!(
                    "laboratory id '{}' contains '/' — ids must be a single path segment",
                    req.id,
                ),
            );
        }
        let mounts: Vec<podman::laboratory::Mount> = req
            .mounts
            .iter()
            .map(|[host, container]| podman::laboratory::Mount {
                host: host.clone(),
                container: container.clone(),
            })
            .collect();
        let env: Vec<(String, String)> = req
            .env
            .iter()
            .map(|[key, value]| (key.clone(), value.clone()))
            .collect();
        let laboratory_binary = self.bin_dir.join("objectiveai-mcp-laboratory");
        if let Err(e) = podman::laboratory::create(
            &self.podman,
            &self.state,
            &self.machine.id,
            &laboratory_binary,
            &req.id,
            &req.image,
            &mounts,
            &env,
            &req.cwd,
            req.agent_full_id.as_deref(),
        )
        .await
        {
            let lower = e.0.to_ascii_lowercase();
            let message = if lower.contains("already in use") || lower.contains("already exists")
            {
                format!("laboratory '{}' already exists", req.id)
            } else {
                format!("create laboratory '{}': {e}", req.id)
            };
            return rpc_err(-32603, message);
        }
        // Echo podman's own record (it carries `created_at`); fall back
        // to the request spec if the read-back races something.
        let identify = match podman::laboratory::list(&self.podman, &self.state).await {
            Ok(labs) => labs
                .into_iter()
                .find(|l| l.id == req.id)
                .map(crate::identify_from_info),
            Err(_) => None,
        }
        .unwrap_or_else(|| Identify {
            id: req.id.clone(),
            image: req.image.clone(),
            mounts: req
                .mounts
                .iter()
                .map(|[host, container]| IdentifyMount {
                    host: host.clone(),
                    container: container.clone(),
                })
                .collect(),
            env: req.env.clone(),
            cwd: req.cwd.clone(),
            created_at: None,
            agent_full_id: req.agent_full_id.clone(),
            // Create never starts the container.
            running: false,
        });
        self.broadcast(&HostNotification::LaboratoryCreated {
            laboratory: identify.clone(),
        })
        .await;
        JsonRpcResult::Ok { result: identify }
    }

    /// `LaboratoryDelete`: retire the lab's server first (its MCP
    /// sessions die with it), force-remove the container (missing is
    /// not an error — podman's `rm -f` semantics), broadcast
    /// `laboratory_deleted`.
    async fn delete_laboratory(
        &self,
        id: &str,
    ) -> JsonRpcResult<TransferAck> {
        self.labs.remove(id);
        // The lab's filetree watch dies with it — abort the pump and
        // drop the materialized tree (daemons clear theirs on the
        // `laboratory_deleted` broadcast below), plus the lifecycle
        // bookkeeping (watch demand and the start/stop lock).
        if let Some((_, pump)) = self.filetree_pumps.remove(id) {
            pump.abort();
        }
        self.filetree.remove(id);
        self.filetree_watchers.remove(id);
        self.lifecycle.remove(id);
        if let Err(e) = podman::laboratory::remove(&self.podman, &self.state, id).await {
            return rpc_err(-32603, format!("delete laboratory '{id}': {e}"));
        }
        self.broadcast(&HostNotification::LaboratoryDeleted { id: id.to_string() })
            .await;
        JsonRpcResult::Ok {
            result: TransferAck {},
        }
    }

    /// `LaboratoryLocalTransfer`: both endpoints live on THIS host
    /// (equal (machine, state) by construction — the proxy only picks
    /// the local variant when the pairs match). Start both containers
    /// if needed and pipe the source's export stream straight into
    /// the destination's import. The host is the only tier allowed to
    /// buffer, and this path does not even stage chunks.
    async fn local_transfer(
        self: &Arc<Self>,
        req: &LocalTransferRequest,
    ) -> JsonRpcResult<LocalTransferResult> {
        let source = match self.lab_server(&req.source_id).await {
            Ok(server) => server,
            Err(message) => {
                return rpc_err(-32603, format!("source '{}': {message}", req.source_id));
            }
        };
        let destination = match self.lab_server(&req.destination_id).await {
            Ok(server) => server,
            Err(message) => {
                return rpc_err(
                    -32603,
                    format!("destination '{}': {message}", req.destination_id),
                );
            }
        };
        match source
            .pipe_export_into(&req.source_path, &destination, &req.destination_path)
            .await
        {
            Ok(bytes) => JsonRpcResult::Ok {
                result: LocalTransferResult { bytes },
            },
            Err(message) => rpc_err(
                -32603,
                format!(
                    "local transfer '{}' -> '{}': {message}",
                    req.source_id, req.destination_id
                ),
            ),
        }
    }

    /// Re-announce one laboratory's CURRENT identity — podman's
    /// record, notably its `running` state — to every connected
    /// daemon, as [`HostNotification::LaboratoryUpdated`]. Called on
    /// every lifecycle transition (lazy start, idle stop) so list
    /// subscribers everywhere hold live state.
    async fn broadcast_updated(&self, id: &str) {
        let identify = match podman::laboratory::list(&self.podman, &self.state).await {
            Ok(labs) => labs
                .into_iter()
                .find(|lab| lab.id == id)
                .map(crate::identify_from_info),
            Err(_) => None,
        };
        if let Some(laboratory) = identify {
            self.broadcast(&HostNotification::LaboratoryUpdated { laboratory })
                .await;
        }
    }

    /// Fan a notification out to every connected daemon; senders whose
    /// channel died are dropped here. Takes `attach_lock` so a
    /// concurrent [`Self::attach_channel`] never loses the change (see
    /// the field docs).
    async fn broadcast(&self, notification: &HostNotification) {
        let Ok(frame) = serde_json::to_string(notification) else {
            return;
        };
        let _guard = self.attach_lock.lock().await;
        self.outbound.retain(|_, tx| tx.send(frame.clone()).is_ok());
    }

    /// Stop every container this host started (initialized cells only)
    /// — the graceful-shutdown path. Stopped, never removed: they and
    /// their filesystems survive for the next host to `start` again.
    pub async fn stop_started(&self) {
        let ids: Vec<String> = self
            .labs
            .iter()
            .filter(|entry| entry.value().get().is_some())
            .map(|entry| entry.key().clone())
            .collect();
        let results = futures::future::join_all(
            ids.iter()
                .map(|id| podman::laboratory::stop(&self.podman, &self.state, id)),
        )
        .await;
        for (id, result) in ids.iter().zip(results) {
            if let Err(e) = result {
                eprintln!("stop laboratory '{id}': {e}");
            }
        }
    }
}

fn rpc_err<T>(code: i64, message: String) -> JsonRpcResult<T> {
    JsonRpcResult::Err { code, message, data: None }
}

/// Build the same-variant error reply for a request the host could not
/// route (no `laboratory_id`, or its laboratory failed to start).
/// Variant names pair 1:1 with the request side; `Drop` has no error
/// shape (infallible ack), so it answers `dropped: false`.
fn reject(
    payload: &RequestPayload,
    code: i64,
    message: String,
) -> ResponsePayload {
    use RequestPayload as Req;
    use ResponsePayload as Resp;
    match payload {
        Req::Initialize => Resp::Initialize(rpc_err(code, message)),
        Req::SessionTerminate => Resp::SessionTerminate(rpc_err(code, message)),
        Req::ToolsList(_) => Resp::ToolsList(rpc_err(code, message)),
        Req::ToolsCall(_) => Resp::ToolsCall(rpc_err(code, message)),
        Req::ResourcesList(_) => Resp::ResourcesList(rpc_err(code, message)),
        Req::ResourcesRead(_) => Resp::ResourcesRead(rpc_err(code, message)),
        // Drop is infallible in shape — nothing was dropped.
        Req::Drop(_) => Resp::Drop(objectiveai_sdk::laboratories::daemon::DropResult {
            dropped: false,
        }),
        Req::Filetree(_) => Resp::Filetree(rpc_err(code, message)),
        Req::ExportBegin(_) => Resp::ExportBegin(rpc_err(code, message)),
        Req::ExportRead(_) => Resp::ExportRead(rpc_err(code, message)),
        Req::ExportAbort(_) => Resp::ExportAbort(rpc_err(code, message)),
        Req::ImportBegin(_) => Resp::ImportBegin(rpc_err(code, message)),
        Req::ImportWrite(_) => Resp::ImportWrite(rpc_err(code, message)),
        Req::ImportEnd(_) => Resp::ImportEnd(rpc_err(code, message)),
        Req::ImportAbort(_) => Resp::ImportAbort(rpc_err(code, message)),
        // Host-level ops are answered in `handle` — reaching here is a
        // routing bug, but the reply shape still pairs correctly.
        Req::Create(_) => Resp::Create(rpc_err(code, message)),
        Req::Delete(_) => Resp::Delete(rpc_err(code, message)),
        Req::LocalTransfer(_) => Resp::LocalTransfer(rpc_err(code, message)),
    }
}
