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
    AgentEphemeralCreateRequest, ChannelRequest, ChannelResponse, CreateRequest,
    EphemeralCreated, ExportChunk, HostIdentify, HostNotification, Identify, IdentifyMount,
    InitializeReply, JsonRpcResult, LocalTransferRequest, LocalTransferResult,
    PluginEphemeralCreateRequest, RequestPayload, ResponsePayload, TransferAck,
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

/// How long to wait before redialing a plugin container's db proxy.
/// The container may still be starting the proxy, or it just went away;
/// either way this is a short retry, not a spin.
const DB_PROXY_REDIAL: std::time::Duration = std::time::Duration::from_millis(250);

/// Stand up a plugin ephemeral's Postgres conduit: keep a WebSocket to
/// the container's injected `objectiveai-db-proxy` and relay it to the
/// owning daemon.
///
/// The host DIALS IN rather than listening. A container cannot reach the
/// machine hosting it — that leg is inbound, and podman's
/// `host.containers.internal` points at an address the WSL provider
/// never answers on — while host→container works already, since it is
/// how the MCP endpoint is reached. So the proxy serves the socket and
/// this dials it, on the port podman publishes.
///
/// TRANSLATION is all this does. The proxy speaks a compact
/// `[tag][id: u32]` framing ([`crate::db_proxy`]); the daemon speaks
/// `HostPostgres` / `PostgresData` keyed by a string `stream_id`. Both
/// carry opaque Postgres bytes, so every payload crosses byte for byte
/// and nothing here parses one. The rules, exhaustively:
///
/// - proxy `Open{id}` → mint a `stream_id`, register the container-write
///   sender, and tell the daemon to dial its Postgres.
/// - proxy `Data{id}` → `PostgresData` on the lane.
/// - proxy `Close{id}` → drop the registration, `HostPostgres::Close` to
///   the daemon, forget the id.
/// - daemon `PostgresData` → routed by `channel.rs` into the sender
///   registered above, whose pump re-frames it for the proxy.
/// - daemon close → `channel.rs` drops that sender, so the pump ends and
///   emits a `Close` frame, which shuts the client socket in-container.
/// - socket death → every stream it carried gets `HostPostgres::Close`,
///   because nothing else would reap the daemon's backends, and a
///   daemon-side connection attached to a dead stream would never be
///   released.
///
/// CONCURRENCY. Postgres pools open many connections at once and they
/// must run in parallel over the one socket:
/// - every send is an UNBOUNDED channel send, so no stream ever waits on
///   another's progress;
/// - the read loop only routes — it never awaits I/O — so a slow stream
///   cannot stall the socket;
/// - ONE writer task owns the sink, so frames interleave rather than
///   queueing behind whichever stream reached it first.
///
/// ONE `watch` cancel governs the dial loop and every task under it.
fn spawn_pg_conduit(
    ws_port: u16,
    channel: u64,
    bridge: Arc<crate::host_command::CommandBridge>,
) -> crate::ephemeral::PgProxy {
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(pg_dial_loop(
        format!("ws://127.0.0.1:{ws_port}/"),
        channel,
        bridge,
        cancel_rx,
    ));
    crate::ephemeral::PgProxy { cancel: cancel_tx }
}

/// Keep a conduit socket to the container for as long as the lab lives.
///
/// Redials on any drop: the proxy accepts a fresh socket without
/// complaint, so a blip costs the in-flight streams (sqlx's pool
/// reconnects) rather than the plugin's database.
async fn pg_dial_loop(
    url: String,
    channel: u64,
    bridge: Arc<crate::host_command::CommandBridge>,
    mut cancel: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        if *cancel.borrow() {
            return;
        }
        let connect = tokio::select! {
            _ = cancel.changed() => return,
            connect = tokio_tungstenite::connect_async(&url) => connect,
        };
        if let Ok((ws, _)) = connect {
            if let tokio_tungstenite::MaybeTlsStream::Plain(tcp) = ws.get_ref() {
                objectiveai_sdk::net::set_tcp_keepalive(tcp);
            }
            pg_relay(ws, channel, Arc::clone(&bridge), cancel.clone()).await;
        }
        tokio::select! {
            _ = cancel.changed() => return,
            _ = tokio::time::sleep(DB_PROXY_REDIAL) => {}
        }
    }
}

/// Relay one conduit socket until it breaks.
async fn pg_relay(
    ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    channel: u64,
    bridge: Arc<crate::host_command::CommandBridge>,
    mut cancel: tokio::sync::watch::Receiver<bool>,
) {
    use crate::db_proxy::Frame;
    use crate::host_command::LaneFrame;
    use futures::{SinkExt as _, StreamExt as _};
    use objectiveai_sdk::binary_frame::WireFrame;
    use objectiveai_sdk::laboratories::daemon::{HostPostgres, PostgresData};
    use tokio_tungstenite::tungstenite::Message;

    let Some(lane) = bridge.outbound.get(&channel).map(|t| t.clone()) else {
        return;
    };
    let (mut sink, mut stream) = ws.split();

    // ONE writer owns the sink; every stream's pump feeds it.
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let writer = tokio::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            if sink.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
    });

    // Streams opened on THIS socket, so they can be torn down with it.
    let mut opened: std::collections::HashMap<u32, String> =
        std::collections::HashMap::new();

    loop {
        let message = tokio::select! {
            _ = cancel.changed() => break,
            message = stream.next() => match message {
                Some(Ok(message)) => message,
                _ => break,
            },
        };
        // Binary is the only shape the format uses; pings are answered
        // by tungstenite itself.
        let Message::Binary(bytes) = message else {
            if matches!(message, Message::Close(_)) {
                break;
            }
            continue;
        };
        // Forward-compat: a frame this build cannot read names nothing
        // it could route.
        let Some(frame) = Frame::decode(&bytes) else {
            continue;
        };
        match frame {
            Frame::Open { id } => {
                let stream_id = uuid::Uuid::new_v4().to_string();
                // The pump that re-frames daemon bytes for this stream.
                // Registered BEFORE the `Open` goes out, so the
                // daemon's first reply cannot arrive with nowhere to go.
                let (write_tx, mut write_rx) =
                    tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
                bridge.register_postgres(stream_id.clone(), channel, write_tx);
                {
                    let out_tx = out_tx.clone();
                    tokio::spawn(async move {
                        while let Some(bytes) = write_rx.recv().await {
                            if out_tx
                                .send(crate::db_proxy::encode_data(id, &bytes))
                                .is_err()
                            {
                                return;
                            }
                        }
                        // The sender was dropped — a daemon-side close,
                        // or this socket's teardown. Either way the
                        // proxy has to shut its client socket, since
                        // nothing else will tell it to.
                        let _ = out_tx.send(crate::db_proxy::encode_close(id));
                    });
                }
                let open = HostPostgres::Open {
                    stream_id: stream_id.clone(),
                };
                let sent = serde_json::to_string(&open)
                    .ok()
                    .and_then(|frame| lane.send(LaneFrame::Text(frame)).ok())
                    .is_some();
                if !sent {
                    bridge.remove_postgres(&stream_id);
                    break;
                }
                opened.insert(id, stream_id);
            }
            Frame::Data { id, bytes } => {
                // Data for a stream we never opened (or already closed)
                // has no `stream_id` to travel under.
                let Some(stream_id) = opened.get(&id) else {
                    continue;
                };
                let data = PostgresData {
                    stream_id: stream_id.clone(),
                    bytes,
                };
                match data.to_wire() {
                    Ok(WireFrame::Binary(frame)) => {
                        if lane.send(LaneFrame::Binary(frame)).is_err() {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            Frame::Close { id } => {
                if let Some(stream_id) = opened.remove(&id) {
                    bridge.remove_postgres(&stream_id);
                    if let Ok(frame) =
                        serde_json::to_string(&HostPostgres::Close { stream_id })
                    {
                        let _ = lane.send(LaneFrame::Text(frame));
                    }
                }
            }
        }
    }

    // The socket is gone. Tell the daemon every stream it carried is
    // over — nothing else will, and a daemon-side Postgres connection
    // left attached to a dead stream would never be reaped.
    for (_, stream_id) in opened {
        bridge.remove_postgres(&stream_id);
        if let Ok(frame) = serde_json::to_string(&HostPostgres::Close { stream_id }) {
            let _ = lane.send(LaneFrame::Text(frame));
        }
    }
    writer.abort();
}

/// The machine-wide server: lazy per-laboratory [`LabServer`]s plus
/// the outbound senders of every live daemon channel (the notification
/// fan-out targets).
pub struct HostServer {
    podman: podman::Podman,
    state: String,
    /// `<objectiveai_dir>/bin` — where the injected binaries live
    /// (create needs them): `objectiveai-mcp-laboratory` for regular
    /// and agent laboratories, `objectiveai-db-proxy` for plugin ones.
    bin_dir: PathBuf,
    machine: MachineIdentity,
    /// Per-laboratory servers. The cell initializes ONCE on the first
    /// routed op (podman `start` + port probe); a failed init leaves it
    /// empty so the next op retries. An INITIALIZED cell marks a
    /// container this host started — [`Self::stop_started`] stops
    /// exactly those.
    labs: DashMap<String, Arc<tokio::sync::OnceCell<Arc<LabServer>>>>,
    /// The live EPHEMERAL laboratories — a fully separate registry
    /// with a fully separate lifetime model (see
    /// [`crate::ephemeral::EphemeralLab`]): registered by the atomic
    /// create+connect op, removed (and the container `rm -f`ed, zero
    /// grace) by [`Self::evaporate`] the moment their single MCP
    /// connection ends. None of the lazy-start/idle-stop machinery
    /// below applies to them.
    ephemerals: DashMap<String, Arc<crate::ephemeral::EphemeralLab>>,
    /// The CONTROL-lane senders (one per connected daemon channel,
    /// keyed by a host-minted registration id) plus the in-flight
    /// host→daemon command exchanges — both live on the shared
    /// [`CommandBridge`](crate::host_command::CommandBridge) so plugin
    /// sessions' executors can hold them without an Arc cycle through
    /// this server. The control lane is unbounded but request-paced —
    /// it carries the identify/auth handshake, attach-time synthesized
    /// snapshots, correlated RPC responses (never droppable), the rare
    /// Created/Updated/Deleted notifications, and now the host-minted
    /// `HostCommandRequest` frames. The filetree fire hose rides
    /// [`Self::filetree_events`] instead. Dead senders are dropped on
    /// the next broadcast.
    bridge: Arc<crate::host_command::CommandBridge>,
    next_outbound: AtomicU64,
    /// The FILETREE lane: pre-serialized `LaboratoryFiletree` frames on
    /// a bounded ring, mirroring the daemon→viewer standard. Each
    /// channel's WS writer holds a receiver and, on `Lagged`, resyncs
    /// itself with fresh snapshots from [`Self::filetree_snapshot_frames`]
    /// — so a stalled daemon socket costs a resync, never unbounded
    /// host memory. The ring retains up to its capacity of frames
    /// (including full-tree snapshots from pump reconnects) while any
    /// receiver lives; that retention is the bounded buffer, which is
    /// the point.
    filetree_events: tokio::sync::broadcast::Sender<String>,
    /// Per-laboratory unified trees as SOURCE SETS
    /// ([`crate::lab_tree::LabTree`]): the container stream plus one
    /// source per mount, registered BEFORE any data flows. Nothing is
    /// ever emitted for an incomplete tree, and every emitted snapshot
    /// is composed from ALL sources — complete and total by
    /// construction. Created only by [`Self::register_lab_tree`],
    /// removed only by delete; every mutation holds `attach_lock`.
    filetree: DashMap<String, crate::lab_tree::LabTree>,
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
    /// Host-side mount watches — the native half of the unified
    /// per-lab filetree (see [`crate::mount_watch`]). Attached at
    /// container start, detached at stop/delete.
    mounts: crate::mount_watch::MountRegistry,
    /// Finished viewer-plugin builds waiting to be drained by the
    /// daemon that asked for them (see [`crate::viewer_build`]).
    builds: crate::viewer_build::BuildArtifacts,
}

impl HostServer {
    pub fn new(bin_dir: PathBuf, state: String, machine: MachineIdentity) -> Self {
        Self {
            podman: podman::Podman::new(bin_dir.clone()),
            state,
            bin_dir,
            machine,
            labs: DashMap::new(),
            ephemerals: DashMap::new(),
            bridge: Arc::new(crate::host_command::CommandBridge::new()),
            next_outbound: AtomicU64::new(0),
            // Capacity matches the kernel's inotify queue
            // (fs.inotify.max_queued_events default 16384) — the ring
            // can absorb everything the container-side watcher can
            // deliver in one burst before its own overflow resync.
            filetree_events: tokio::sync::broadcast::channel(16384).0,
            filetree: DashMap::new(),
            filetree_pumps: DashMap::new(),
            filetree_watchers: DashMap::new(),
            lifecycle: DashMap::new(),
            attach_lock: tokio::sync::Mutex::new(()),
            mounts: crate::mount_watch::MountRegistry::default(),
            builds: crate::viewer_build::BuildArtifacts::default(),
        }
    }

    /// Attach a freshly-connected daemon channel: enqueue its
    /// [`HostIdentify`] (built from podman's CURRENT laboratory set —
    /// podman is the only source of truth, nothing mirrored to drift)
    /// and the auth envelope as the channel's first two frames, then
    /// register its CONTROL sender for notification fan-out and
    /// subscribe it to the filetree ring — all under `attach_lock`,
    /// atomically against broadcasts and the pumps' folds (see the
    /// field docs). Exactly-once for filetree state: the synthesized
    /// snapshots are queued on `reply_tx` in the SAME lock hold as the
    /// ring `subscribe()`, and every fold+send in
    /// [`Self::filetree_event`] holds the same lock — so an event is
    /// either folded into this channel's snapshot or delivered by its
    /// ring receiver, never neither. The channel's writer drains the
    /// control lane first (biased), so the snapshots reach the wire
    /// before any ring delta. Returns the id to
    /// [`Self::detach_channel`] with, plus the channel's ring receiver.
    pub async fn attach_channel(
        &self,
        reply_tx: tokio::sync::mpsc::UnboundedSender<crate::host_command::LaneFrame>,
        auth_frame: String,
    ) -> (u64, tokio::sync::broadcast::Receiver<String>) {
        use crate::host_command::LaneFrame;
        let _guard = self.attach_lock.lock().await;
        let identify = HostIdentify {
            state: self.state.clone(),
            machine: self.machine.clone(),
            laboratories: self.identify_all().await,
        };
        let identify_frame =
            serde_json::to_string(&identify).expect("identify serializes");
        let _ = reply_tx.send(LaneFrame::Text(identify_frame));
        let _ = reply_tx.send(LaneFrame::Text(auth_frame));
        // A synthesized file-tree snapshot per WATCHED laboratory, so
        // this daemon's materialized trees start current — the same
        // snapshot-then-deltas contract the lab endpoint itself opens
        // with. Under `attach_lock`, atomic against the pumps' folds.
        for frame in self.filetree_snapshot_frames() {
            let _ = reply_tx.send(LaneFrame::Text(frame));
        }
        let filetree_rx = self.filetree_events.subscribe();
        let id = self.next_outbound.fetch_add(1, Ordering::Relaxed);
        self.bridge.outbound.insert(id, reply_tx);
        (id, filetree_rx)
    }

    /// The shared command bridge — the channel reader routes inbound
    /// [`HostCommandResponse`](objectiveai_sdk::laboratories::daemon::HostCommandResponse)
    /// frames through it.
    pub fn bridge(&self) -> &Arc<crate::host_command::CommandBridge> {
        &self.bridge
    }

    /// One synthesized `LaboratoryFiletree` Snapshot frame per
    /// materialized laboratory tree — the resync currency shared by
    /// [`Self::attach_channel`] (under `attach_lock`) and each
    /// channel writer's `Lagged` recovery (deliberately WITHOUT the
    /// lock): the caller's ring receiver is already subscribed, so
    /// every event folded before this clone is in the snapshot and
    /// every later one is still in-order in its ring; a delta replayed
    /// on top of a newer snapshot is an idempotent re-apply. DashMap's
    /// entry locking keeps the clone un-torn against a concurrent
    /// fold.
    pub(crate) fn filetree_snapshot_frames(&self) -> Vec<String> {
        self.filetree
            .iter()
            .filter_map(|entry| {
                // Incomplete trees are simply absent — the daemon has
                // never seen them, and must not until they compose.
                let children = entry.value().compose()?;
                serde_json::to_string(&HostNotification::LaboratoryFiletree {
                    id: entry.key().clone(),
                    event: FileTreeEvent::Snapshot { children },
                })
                .ok()
            })
            .collect()
    }

    /// Register (or re-register, on restart) a laboratory's SOURCE SET
    /// — one entry per mount — BEFORE its container pump spawns and
    /// before any delivery notifier runs, so no source data can ever
    /// race its own registration. Re-registration only adds missing
    /// sources: existing ones keep their old watch and delivered flag,
    /// so a frozen view stays complete across restarts until fresh
    /// walks re-deliver.
    async fn register_lab_tree(
        &self,
        id: &str,
        sources: Vec<(Vec<String>, Arc<crate::mount_watch::MountWatch>)>,
    ) {
        let _guard = self.attach_lock.lock().await;
        match self.filetree.entry(id.to_string()) {
            dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                entry.get_mut().merge_sources(sources);
            }
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                entry.insert(crate::lab_tree::LabTree::new(sources));
            }
        }
    }

    /// Emit the lab's COMPOSED snapshot iff its source set is
    /// complete. Must run under `attach_lock`; takes the map entry
    /// guard briefly (clone the children, drop the guard, THEN
    /// serialize — a whole-tree serde run must not hold the shard
    /// lock) and sends inside the same `attach_lock` hold (a send
    /// outside it could be clobbered daemon-side by an interleaved
    /// delta's frame).
    fn try_emit_composed(&self, id: &str) {
        let Some(children) = self.filetree.get(id).and_then(|tree| tree.compose()) else {
            return;
        };
        let Ok(frame) = serde_json::to_string(&HostNotification::LaboratoryFiletree {
            id: id.to_string(),
            event: FileTreeEvent::Snapshot { children },
        }) else {
            return;
        };
        let _ = self.filetree_events.send(frame);
    }

    /// The container source's snapshot (called by the lab's pump on
    /// every SSE connect): store it, and emit the composed snapshot if
    /// that completed — or refreshed — the set. A container reconnect
    /// therefore re-emits a COMPLETE tree; mounts can never flicker.
    pub(crate) async fn source_container_snapshot(
        &self,
        id: &str,
        children: Vec<FileTreeNode>,
    ) {
        let _guard = self.attach_lock.lock().await;
        {
            // Unregistered lab (delete raced the pump) — drop.
            let Some(mut entry) = self.filetree.get_mut(id) else {
                return;
            };
            entry.set_container(children);
        }
        self.try_emit_composed(id);
    }

    /// A container source delta: fold it, and pass it through verbatim
    /// ONLY when the lab's tree is complete — an incomplete tree emits
    /// nothing (the delta still folds, so it rides the eventual
    /// composed snapshot).
    pub(crate) async fn source_container_delta(&self, id: &str, event: FileTreeEvent) {
        let Ok(frame) = serde_json::to_string(&HostNotification::LaboratoryFiletree {
            id: id.to_string(),
            event: event.clone(),
        }) else {
            return;
        };
        let _guard = self.attach_lock.lock().await;
        let complete = {
            let Some(mut entry) = self.filetree.get_mut(id) else {
                return;
            };
            if !entry.fold_container(event) {
                // No container snapshot yet — a baseless delta.
                return;
            }
            entry.complete()
        };
        if complete {
            let _ = self.filetree_events.send(frame);
        }
    }

    /// A mount source delivered (initial walk done, resync re-walk, or
    /// a restarted watch's fresh walk): mark it — swapping the lab's
    /// source to `watch` at THIS moment, never earlier — and emit the
    /// composed snapshot if the set is (still or newly) complete.
    pub(crate) async fn source_mount_delivered(
        &self,
        id: &str,
        mountpoint: &[String],
        watch: &Arc<crate::mount_watch::MountWatch>,
    ) {
        let _guard = self.attach_lock.lock().await;
        {
            let Some(mut entry) = self.filetree.get_mut(id) else {
                return;
            };
            if !entry.mount_delivered(mountpoint, watch) {
                return;
            }
        }
        self.try_emit_composed(id);
    }

    /// A mount source delta, already in lab space (mountpoint-prefixed
    /// path): pass-through ONLY when the lab's tree is complete AND
    /// this mountpoint's source has delivered. Dropping is always
    /// safe: the pump folds the mount's cached tree BEFORE emitting,
    /// and compose reads that tree live under this same lock — every
    /// dropped delta is inside the next composed snapshot.
    pub(crate) async fn source_mount_delta(
        &self,
        id: &str,
        mountpoint: &[String],
        event: FileTreeEvent,
    ) {
        let Ok(frame) = serde_json::to_string(&HostNotification::LaboratoryFiletree {
            id: id.to_string(),
            event,
        }) else {
            return;
        };
        let _guard = self.attach_lock.lock().await;
        let pass = self
            .filetree
            .get(id)
            .is_some_and(|tree| tree.complete() && tree.mount_ready(mountpoint));
        if pass {
            let _ = self.filetree_events.send(frame);
        }
    }

    /// Detach a disconnected daemon channel: drop its notification
    /// sender and withdraw ALL of its demand — its filetree watches
    /// and its MCP sessions — then schedule the idle check for every
    /// laboratory it touched. A dead daemon never pins a container.
    pub fn detach_channel(self: &Arc<Self>, id: u64) {
        self.bridge.outbound.remove(&id);
        // Its in-flight command exchanges can never complete — fail
        // them (their streams end, which consumers read as done).
        self.bridge.detach(id);
        // Its ephemerals' single connections are unreachable forever —
        // evaporate each one (zero grace, rm -f).
        // CHANNEL-CONDITIONAL: these spawns queue on each id's
        // lifecycle mutex, so a stale sweep entry can land AFTER a
        // successor conduit re-created the same id — it must not
        // evaporate the fresh container. (An ephemeral created AFTER
        // this snapshot never leaks either: `finish_ephemeral`
        // re-checks the channel's outbound sender post-insert — see
        // its tail — and self-cleans if we already detached it.)
        let orphaned: Vec<String> = self
            .ephemerals
            .iter()
            .filter(|entry| entry.value().channel == id)
            .map(|entry| entry.key().clone())
            .collect();
        for lab_id in orphaned {
            let host = Arc::clone(self);
            tokio::spawn(async move {
                host.evaporate_if_channel(&lab_id, id).await;
            });
        }
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
            RequestPayload::AgentEphemeralCreate(req) => {
                let result = self
                    .create_agent_ephemeral(channel, &request.headers, req)
                    .await;
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::AgentEphemeralCreate(result),
                };
            }
            RequestPayload::PluginEphemeralCreate(req) => {
                let result = self
                    .create_plugin_ephemeral(channel, &request.headers, req)
                    .await;
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::PluginEphemeralCreate(result),
                };
            }
            RequestPayload::PluginImageReset(req) => {
                let result = self.reset_plugin_image(req).await;
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::PluginImageReset(result),
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
            // Host-level: a build laboratory is created, waited on and
            // removed inside this ONE request, and its artifact is
            // parked host-wide — there is never an envelope lab to
            // demux to.
            RequestPayload::BuildCreate(req) => {
                let result = self.build_viewer_plugin(req).await;
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::BuildCreate(result),
                };
            }
            RequestPayload::BuildRead(req) => {
                let result = match self.builds.read(&req.transfer_id).await {
                    Ok((data, eof)) => JsonRpcResult::Ok {
                        // Raw bytes — they ride OUT OF BAND in the
                        // channel's binary wire frame.
                        result: ExportChunk { data, eof },
                    },
                    Err(message) => rpc_err(-32603, message),
                };
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::BuildRead(result),
                };
            }
            RequestPayload::BuildAbort(req) => {
                self.builds.discard(&req.transfer_id).await;
                return ChannelResponse {
                    id: request.id,
                    payload: ResponsePayload::BuildAbort(JsonRpcResult::Ok {
                        result: TransferAck {},
                    }),
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
                // Live ephemerals ack watch state without registering
                // anything: watches never drive their lifetime, and
                // the daemon's edge-triggered reconnect replay must
                // stay harmless. (A dead ephemeral falls through and
                // fails at the regular lazy start — the lab is gone.)
                if self.ephemerals.contains_key(&lab_id) {
                    return ChannelResponse {
                        id: request.id,
                        payload: ResponsePayload::Filetree(JsonRpcResult::Ok {
                            result: TransferAck {},
                        }),
                    };
                }
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
        // EPHEMERAL demux: the lifetime-ending ops evaporate at THIS
        // layer (it owns the registry); everything else routes to the
        // lab. Early return — none of the regular idle machinery
        // below applies.
        if let Some(lab) = self.ephemerals.get(&lab_id).map(|e| Arc::clone(e.value())) {
            let payload = match &request.payload {
                RequestPayload::SessionTerminate => {
                    // The one connection's owner is the only party who
                    // may end the laboratory.
                    let owns = crate::upstream::response_id_from_headers(&request.headers)
                        .as_deref()
                        == Some(lab.response_id.as_str());
                    if owns {
                        self.evaporate(&lab_id).await;
                        ResponsePayload::SessionTerminate(JsonRpcResult::Ok {
                            result: (),
                        })
                    } else {
                        ResponsePayload::SessionTerminate(rpc_err(
                            -32001,
                            "response id does not own this ephemeral laboratory".into(),
                        ))
                    }
                }
                RequestPayload::Drop(req) => {
                    if req.response_id == lab.response_id {
                        self.evaporate(&lab_id).await;
                        ResponsePayload::Drop(
                            objectiveai_sdk::laboratories::daemon::DropResult {
                                dropped: true,
                            },
                        )
                    } else {
                        ResponsePayload::Drop(
                            objectiveai_sdk::laboratories::daemon::DropResult {
                                dropped: false,
                            },
                        )
                    }
                }
                _ => return lab.handle(request).await,
            };
            return ChannelResponse {
                id: request.id,
                payload,
            };
        }
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

    /// Acquire the id's lifecycle mutex, REVALIDATED: terminal paths
    /// (`evaporate`, `delete_laboratory`) remove the map entry under
    /// their own lock, so a waiter can wake holding an ORPHANED mutex
    /// while a fresh one was minted — its critical section would then
    /// run in parallel with the fresh mutex's holder. Loop until the
    /// mutex we hold is the one the map serves (`Arc::ptr_eq`); every
    /// lifecycle critical section MUST acquire through here.
    ///
    /// Returns the Arc too: terminal paths pass it to
    /// `remove_if(ptr_eq)` so they only ever remove the entry they
    /// hold.
    async fn lock_lifecycle(
        &self,
        id: &str,
    ) -> (Arc<tokio::sync::Mutex<()>>, tokio::sync::OwnedMutexGuard<()>) {
        loop {
            let mutex = self.lifecycle(id);
            let guard = mutex.clone().lock_owned().await;
            if self
                .lifecycle
                .get(id)
                .is_some_and(|entry| Arc::ptr_eq(entry.value(), &mutex))
            {
                return (mutex, guard);
            }
            // Entry removed/replaced while we waited — retry on the
            // live one (a freshly minted Arc always validates; no
            // livelock).
        }
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
        let (_lock, _guard) = self.lock_lifecycle(id).await;
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
        self.mounts.detach_lab(id);
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
        let (_lock, _guard) = self.lock_lifecycle(id).await;
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
            // The lab's label record decides how to serve it: a plugin
            // lab publishes ITS manifest port (recorded at create) and
            // connects with the command-forwarding executor; a regular
            // lab publishes the fixed LAB_PORT and gets the filetree
            // pump (plugin containers run the image's own entrypoint —
            // no injected MCP, no /filetree surface).
            // The record and the port table together: `host_ports`
            // returns every mapping, so it does not need to be told which
            // internal port to ask about, which is what makes it
            // independent of the record and lets both podman calls run at
            // once instead of one behind the other.
            let (listed, ports) = tokio::join!(
                podman::laboratory::list(&self.podman, &self.state),
                podman::laboratory::host_ports(&self.podman, &self.state, id),
            );
            let lab = listed
                .ok()
                .and_then(|labs| labs.into_iter().find(|lab| lab.id == id));
            // Mounts are kept, not just the plugin: the filetree pump
            // below needs them, and this is the read that has them.
            let (plugin, mounts) = match lab {
                Some(lab) => (lab.plugin, lab.mounts),
                None => (None, Vec::new()),
            };
            let internal_port = plugin
                .as_ref()
                .map(|p| p.port)
                .unwrap_or(podman::laboratory::LAB_PORT);
            match ports.and_then(|ports| {
                ports.get(&internal_port).copied().ok_or_else(|| {
                    podman::Error(format!("no mapping for {internal_port}/tcp"))
                })
            }) {
                Ok(port) => {
                    let base_url = format!("http://127.0.0.1:{port}");
                    let seed = plugin.map(|p| crate::server::PluginSeed {
                        bridge: Arc::clone(&self.bridge),
                        plugin: objectiveai_sdk::mcp::server::Plugin {
                            owner: p.owner,
                            name: p.name,
                            version: p.version,
                        },
                    });
                    if seed.is_none() {
                        self.spawn_filetree_pump(id, &base_url, &mounts).await;
                    }
                    Ok(Arc::new(LabServer::new(
                        id.to_string(),
                        base_url,
                        Arc::clone(&self.bridge),
                        seed,
                    )))
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
    /// `mounts` comes from the caller's own read of podman's record
    /// (the container label — the source of truth, surviving host
    /// restarts). Passed in rather than read here because every caller
    /// has already listed the container for its own reasons, and a
    /// second `podman ps` for the same record is a whole process launch
    /// spent re-learning what the caller knows.
    async fn spawn_filetree_pump(
        self: &Arc<Self>,
        id: &str,
        base_url: &str,
        mounts: &[podman::laboratory::Mount],
    ) {
        // Strict order, each step before the next: (1) attach the
        // watches (subscriptions + walks, NO delivery notifiers yet),
        // (2) register the lab's SOURCE SET, (3) spawn the delivery
        // notifiers, (4) spawn the container pump — so no source can
        // deliver before its registration exists, and the tree cannot
        // complete before every source is declared. Container paths
        // are POSIX strings — split on '/', never host Path semantics.
        let mut sources: Vec<(Vec<String>, Arc<crate::mount_watch::MountWatch>)> = Vec::new();
        for mount in mounts {
            let mountpoint: Vec<String> = mount
                .container
                .split('/')
                .filter(|c| !c.is_empty())
                .map(String::from)
                .collect();
            if let Some(watch) = self
                .mounts
                .attach(self, id, &mount.host, mountpoint.clone())
                .await
            {
                sources.push((mountpoint, watch));
            }
        }
        self.register_lab_tree(id, sources.clone()).await;
        for (mountpoint, watch) in sources {
            let host = Arc::clone(self);
            let lab_id = id.to_string();
            tokio::spawn(async move {
                watch.ready().await;
                host.source_mount_delivered(&lab_id, &mountpoint, &watch).await;
            });
        }
        let handle = tokio::spawn(crate::filetree::pump(
            Arc::clone(self),
            id.to_string(),
            base_url.to_string(),
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
        // Agent and plugin laboratories are EPHEMERAL now — created
        // exclusively by the atomic create+connect ops, never via
        // `Create`. Any agent provenance or reserved-prefix claim on
        // this path is rejected authoritatively, whatever daemon sent
        // it.
        if req.agent_full_id.is_some() {
            return rpc_err(
                -32602,
                format!(
                    "laboratory '{}' carries agent_full_id — agent laboratories are ephemeral, created via agent_ephemeral_create",
                    req.id,
                ),
            );
        }
        if req
            .id
            .starts_with(objectiveai_sdk::agent::AGENT_LABORATORY_ID_PREFIX)
        {
            return rpc_err(
                -32602,
                format!(
                    "laboratory id '{}' uses the reserved agent-laboratory prefix '{}'",
                    req.id,
                    objectiveai_sdk::agent::AGENT_LABORATORY_ID_PREFIX,
                ),
            );
        }
        if req
            .id
            .starts_with(objectiveai_sdk::laboratories::PLUGIN_LABORATORY_ID_PREFIX)
        {
            return rpc_err(
                -32602,
                format!(
                    "laboratory id '{}' uses the reserved plugin-laboratory prefix '{}' — plugin laboratories are ephemeral, created via plugin_ephemeral_create",
                    req.id,
                    objectiveai_sdk::laboratories::PLUGIN_LABORATORY_ID_PREFIX,
                ),
            );
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
            agent_full_id: None,
            plugin: None,
            response_id: None,
            // Create never starts the container.
            running: false,
        });
        self.broadcast(&HostNotification::LaboratoryCreated {
            laboratory: identify.clone(),
        })
        .await;
        JsonRpcResult::Ok { result: identify }
    }

    /// `AgentEphemeralCreate`: the atomic create+connect for an
    /// EPHEMERAL agent laboratory. See [`Self::finish_ephemeral`] for
    /// the shared back half; the front half is the agent-specific
    /// identity + image ensure.
    async fn create_agent_ephemeral(
        self: &Arc<Self>,
        channel: u64,
        headers: &indexmap::IndexMap<String, String>,
        req: &AgentEphemeralCreateRequest,
    ) -> JsonRpcResult<EphemeralCreated> {
        if let Err(message) = validate_response_id(&req.response_id) {
            return rpc_err(-32602, message);
        }
        if req.agent_full_id.is_empty() {
            return rpc_err(-32602, "`agent_full_id` cannot be empty".into());
        }
        if let Err(message) = req.laboratory.image.validate() {
            return rpc_err(-32602, format!("image: {message}"));
        }
        let derived = objectiveai_sdk::agent::laboratories::derived_id(
            &req.agent_full_id,
            &req.laboratory,
        );
        let id = format!("{derived}-{}", req.response_id);
        // Serialize the whole create under the per-id lifecycle lock —
        // a crashed/retried sibling create for the same response id
        // waits its turn and then sees (and removes) the stale
        // container.
        let (_lock, _guard) = self.lock_lifecycle(&id).await;
        let resolved = match podman::laboratory::ensure_agent_image(
            &self.podman,
            &derived,
            &req.laboratory.image,
        )
        .await
        {
            Ok(resolved) => resolved,
            Err(e) => return rpc_err(-32603, format!("ensure agent image: {e}")),
        };
        // Stale duplicate (crash/retry — response ids are unique per
        // completion): evaporate + recreate fresh. The remove is
        // unconditional and idempotent, covering containers that
        // survived a host restart without a map entry. LOCKED variant:
        // we hold this id's lifecycle mutex, and the id LIVES ON with
        // the new container — the lifecycle entry must stay.
        self.evaporate_locked(&id).await;
        if let Err(e) = podman::laboratory::remove(&self.podman, &self.state, &id).await
        {
            return rpc_err(-32603, format!("remove stale ephemeral '{id}': {e}"));
        }
        let env: Vec<(String, String)> = req
            .laboratory
            .env
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|[key, value]| (key, value))
            .collect();
        let cwd = req.laboratory.cwd.clone().unwrap_or_else(|| "/".to_string());
        let laboratory_binary = self.bin_dir.join("objectiveai-mcp-laboratory");
        let identity_env =
            objectiveai_sdk::identity::Identity::from_transient_headers(
                headers,
            )
            .identity_env();
        if let Err(e) = podman::laboratory::create_agent_ephemeral(
            &self.podman,
            &self.state,
            &self.machine.id,
            &laboratory_binary,
            &id,
            &req.laboratory.image,
            &resolved,
            &env,
            &identity_env,
            &cwd,
            &req.agent_full_id,
            &req.response_id,
        )
        .await
        {
            return rpc_err(-32603, format!("create ephemeral '{id}': {e}"));
        }
        self.finish_ephemeral(
            channel,
            headers,
            &id,
            &req.response_id,
            podman::laboratory::LAB_PORT,
            None,
            true,
            false, // agent ephemerals get no database, so no proxy
        )
        .await
    }

    /// `PluginEphemeralCreate`: the plugin twin —
    /// [`crate::plugin_image::ensure`] (exists-fast-path, else
    /// bin-locked clone+build+tag) supplies the image and the
    /// manifest port, then the shared back half connects with the
    /// command-forwarding executor.
    async fn create_plugin_ephemeral(
        self: &Arc<Self>,
        channel: u64,
        headers: &indexmap::IndexMap<String, String>,
        req: &PluginEphemeralCreateRequest,
    ) -> JsonRpcResult<EphemeralCreated> {
        if let Err(message) = validate_response_id(&req.response_id) {
            return rpc_err(-32602, message);
        }
        let coords = match crate::plugin_image::PluginCoords::canonicalize(
            &req.owner,
            &req.name,
            &req.version,
        ) {
            Ok(coords) => coords,
            Err(message) => return rpc_err(-32602, message),
        };
        let id = coords.ephemeral_laboratory_id(&req.response_id);
        let (_lock, _guard) = self.lock_lifecycle(&id).await;
        // DEVELOPMENT: the daemon only ever forwards a registered
        // plugin to the LOCAL host, so this path is one this process
        // can see. A bad registration is the developer's own mistake,
        // not an internal fault — hence its own error code.
        let development = req.development.as_ref().map(std::path::Path::new);
        if let Some(dir) = development
            && let Err(e) = crate::plugin_image::check_development_dir(dir).await
        {
            return rpc_err(
                objectiveai_sdk::laboratories::daemon::PLUGIN_DEVELOPMENT_SOURCE_CODE,
                e.0,
            );
        }
        let ensured = match crate::plugin_image::ensure(
            &self.podman,
            &self.bin_dir,
            &coords,
            development,
        )
        .await
        {
            Ok(ensured) => ensured,
            Err(message) => {
                return rpc_err(
                    -32603,
                    format!(
                        "ensure plugin image {}/{}@{}: {message}",
                        coords.owner, coords.name, coords.version,
                    ),
                );
            }
        };
        // Stale duplicate: evaporate + recreate fresh (see the agent
        // twin for the rationale — LOCKED variant, entry stays).
        self.evaporate_locked(&id).await;
        if let Err(e) = podman::laboratory::remove(&self.podman, &self.state, &id).await
        {
            return rpc_err(-32603, format!("remove stale ephemeral '{id}': {e}"));
        }
        let label = podman::laboratory::PluginLabel {
            owner: coords.owner.clone(),
            name: coords.name.clone(),
            version: coords.version.clone(),
            port: ensured.port,
            sha: ensured.sha,
        };
        // Identity env: the six agent values from the request headers,
        // PLUS the plugin trio from the CANONICAL coordinates — this
        // authenticated create is the trio's authority (wire-parsed
        // bags always null it) — PLUS the plugin's own declared
        // arguments and the Postgres URL the container dials
        // (role/password/database from the daemon; the address is the
        // injected db proxy, on loopback INSIDE the container).
        let identity_env = {
            let mut args =
                objectiveai_sdk::identity::Identity::from_transient_headers(
                    headers,
                );
            args.plugin_owner = Some(coords.owner.clone());
            args.plugin_name = Some(coords.name.clone());
            args.plugin_version = Some(coords.version.clone());
            let mut env = args.identity_env();
            // The plugin's declared arguments, verbatim off the header
            // that already carries them to its MCP server on every
            // call. Same JSON, same name minus the `X-` — so a plugin
            // can read its configuration at startup instead of waiting
            // for a call to hand it over.
            //
            // Safe to freeze into the environment ONLY because this is
            // an EPHEMERAL container: one completion per container, so
            // the arguments cannot change under it. Absent when the
            // agent declared none — never an empty string.
            if let Some(arguments) = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-ARGUMENTS"))
                .map(|(_, v)| v.clone())
            {
                env.push(("OBJECTIVEAI_ARGUMENTS".to_string(), arguments));
            }
            let enc = |s: &str| {
                percent_encoding::utf8_percent_encode(
                    s,
                    percent_encoding::NON_ALPHANUMERIC,
                )
                .to_string()
            };
            // An ordinary local Postgres server, as far as the plugin is
            // concerned — which is the entire point of the proxy. The
            // port is FIXED, so this URL can be stamped here, before
            // anything is running to be asked about it.
            env.push((
                "OBJECTIVEAI_POSTGRES_URL".to_string(),
                format!(
                    "postgres://{}:{}@127.0.0.1:{}/{}",
                    enc(&req.db_role),
                    enc(&req.db_password),
                    podman::laboratory::DB_PROXY_PG_PORT,
                    enc(&req.db_database),
                ),
            ));
            env
        };
        if let Err(e) = podman::laboratory::create_plugin(
            &self.podman,
            &self.state,
            &id,
            &coords.image(),
            &label,
            &req.response_id,
            &identity_env,
            &self.bin_dir.join(podman::laboratory::DB_PROXY_BINARY),
        )
        .await
        {
            return rpc_err(-32603, format!("create ephemeral '{id}': {e}"));
        }
        let plugin = objectiveai_sdk::mcp::server::Plugin {
            owner: coords.owner.clone(),
            name: coords.name.clone(),
            version: coords.version.clone(),
        };
        self.finish_ephemeral(
            channel,
            headers,
            &id,
            &req.response_id,
            ensured.port,
            Some(plugin),
            false,
            true,
        )
        .await
    }

    /// The shared back half of both ephemeral creates: the container
    /// EXISTS (created, not started) — start it, resolve the
    /// published port, open THE single MCP connection with the
    /// request's headers (they carry the full agent-argument set the
    /// in-container server requires), spawn the filetree pump (agent
    /// kind only), register + broadcast, and reply with identity AND
    /// initialize result. ANY failure past this point removes the
    /// container — a half-made ephemeral is never left behind.
    #[allow(clippy::too_many_arguments)]
    async fn finish_ephemeral(
        self: &Arc<Self>,
        channel: u64,
        headers: &indexmap::IndexMap<String, String>,
        id: &str,
        response_id: &str,
        internal_port: u16,
        plugin: Option<objectiveai_sdk::mcp::server::Plugin>,
        filetree: bool,
        db_proxy: bool,
    ) -> JsonRpcResult<EphemeralCreated> {
        let fail = |message: String| async move {
            let _ = podman::laboratory::remove(&self.podman, &self.state, id).await;
            rpc_err(-32603, message)
        };
        if let Err(e) = podman::laboratory::start(&self.podman, &self.state, id).await {
            return fail(format!("start ephemeral '{id}': {e}")).await;
        }
        // The container's published ports, and the proxy started, in one
        // round trip. `host_ports` returns the whole table, so the MCP
        // port and the proxy's come from a single `podman port`; the exec
        // is independent of both, since the proxy needs nothing passed to
        // it, so it goes alongside rather than after.
        //
        // The proxy is started BEFORE the MCP connect so it is attaching
        // while MCP initializes. A failure fails the create, like every
        // step past start — the database is mandatory for a plugin. But
        // the DIAL is not gated: the proxy holds an accepted client until
        // a host attaches, so a plugin that connects first simply waits
        // in its `connect` instead of failing. There is no race to gate.
        let (ports, ()) = match tokio::try_join!(
            async {
                podman::laboratory::host_ports(&self.podman, &self.state, id)
                    .await
                    .map_err(|e| format!("ephemeral '{id}' ports: {e}"))
            },
            async {
                if !db_proxy {
                    return Ok(());
                }
                podman::laboratory::start_db_proxy(&self.podman, &self.state, id)
                    .await
                    .map_err(|e| format!("start db proxy in ephemeral '{id}': {e}"))
            },
        ) {
            Ok(both) => both,
            Err(message) => return fail(message).await,
        };
        let Some(port) = ports.get(&internal_port).copied() else {
            return fail(format!(
                "ephemeral '{id}': no published port for {internal_port}/tcp"
            ))
            .await;
        };
        let base_url = format!("http://127.0.0.1:{port}");
        let pg = if db_proxy {
            match ports
                .get(&podman::laboratory::DB_PROXY_WS_PORT)
                .copied()
            {
                Some(ws_port) => {
                    Some(spawn_pg_conduit(ws_port, channel, Arc::clone(&self.bridge)))
                }
                None => {
                    return fail(format!(
                        "ephemeral '{id}': no published port for the db proxy ({}/tcp)",
                        podman::laboratory::DB_PROXY_WS_PORT
                    ))
                    .await;
                }
            }
        } else {
            None
        };
        // THE connection — same executor construction as
        // LabServer::initialize: plugin ephemerals get the real
        // command-forwarder reading the live header bag, agent
        // ephemerals the inert form.
        let (executor, transient) = match plugin {
            Some(plugin) => {
                let transient =
                    Arc::new(tokio::sync::RwLock::new(headers.clone()));
                (
                    crate::host_command::HostCommandExecutor {
                        inner: Some(Arc::new(
                            crate::host_command::PluginExecutorState {
                                bridge: Arc::clone(&self.bridge),
                                plugin,
                                channel,
                                transient: Arc::clone(&transient),
                            },
                        )),
                    },
                    Some(transient),
                )
            }
            None => (
                crate::host_command::HostCommandExecutor { inner: None },
                None,
            ),
        };
        // THE connection, and podman's record of the container, at the
        // same time. The record is keyed by id and carries nothing the
        // connection produces, while the two cost quite different things
        // — an HTTP initialize round trip against a process launch — so
        // there is no reason for either to wait on the other. ONE read
        // serves both consumers below: the filetree pump's mounts and the
        // `Identify` echo.
        let client = crate::upstream::lab_mcp_client().with_executor(executor);
        let (connected, listed) = tokio::join!(
            client.connect(
                format!("{base_url}/"),
                None,
                Some(crate::upstream::sanitize_connect_headers(headers)),
            ),
            podman::laboratory::list(&self.podman, &self.state),
        );
        let connection = match connected {
            Ok(connection) => connection,
            Err(e) => return fail(format!("connect ephemeral '{id}': {e}")).await,
        };
        let lab = listed
            .ok()
            .and_then(|labs| labs.into_iter().find(|lab| lab.id == id));
        // First hop of the list-changed relay — installed while we
        // still hold the connection, before `EphemeralLab::new`
        // consumes it. Captures only a sender + frame string
        // (cycle-safety rule — see the helper's docs).
        crate::upstream::install_list_changed_forwarders(
            &self.bridge,
            channel,
            id,
            response_id,
            &connection,
        );
        let mcp_session_id = connection.session_id.clone();
        let initialize_result = connection.initialize_result.clone();
        // Agent ephemerals carry the injected MCP — proxy its
        // /filetree like any regular lab (observation only, never
        // lifetime demand).
        if filetree {
            let mounts = lab.as_ref().map(|lab| lab.mounts.as_slice()).unwrap_or(&[]);
            self.spawn_filetree_pump(id, &base_url, mounts).await;
        }
        // Echo podman's own record (it carries `created_at` +
        // `response_id`); fall back to a minimal identity if the
        // read-back races something.
        let identify = lab
            .map(|lab| {
                let mut identify = crate::identify_from_info(lab);
                // The container was started microseconds ago; the ps
                // read-back may race the state flip.
                identify.running = true;
                identify
            })
            .unwrap_or_else(|| Identify {
                id: id.to_string(),
                image: objectiveai_sdk::laboratories::LaboratoryImage::Registry(
                    objectiveai_sdk::laboratories::RegistryLaboratoryImage {
                        registry: "localhost".to_string(),
                        name: "objectiveai-ephemeral".to_string(),
                        pin: objectiveai_sdk::laboratories::LaboratoryImagePin::Tag(
                            "unknown".to_string(),
                        ),
                    },
                ),
                mounts: Vec::new(),
                env: Vec::new(),
                cwd: "/".to_string(),
                created_at: None,
                agent_full_id: None,
                plugin: None,
                response_id: Some(response_id.to_string()),
                running: true,
            });
        let lab = Arc::new(crate::ephemeral::EphemeralLab::new(
            response_id.to_string(),
            channel,
            base_url,
            connection,
            transient,
            pg,
        ));
        self.ephemerals.insert(id.to_string(), lab);
        // Detach-race net: `detach_channel` removes the channel's
        // outbound sender FIRST, then snapshots `ephemerals` — an
        // entry we insert after its snapshot would leak forever. If
        // the sender is still present here, any later detach scan runs
        // after our insert and finds the entry (its evaporate queues
        // behind the lifecycle lock we hold); if it is already gone,
        // WE are the only ones who know this container exists —
        // self-clean and fail the create.
        if !self.bridge.outbound.contains_key(&channel) {
            self.evaporate_locked(id).await;
            return rpc_err(
                -32603,
                format!("ephemeral '{id}': owning daemon channel disconnected"),
            );
        }
        self.broadcast(&HostNotification::LaboratoryCreated {
            laboratory: identify.clone(),
        })
        .await;
        JsonRpcResult::Ok {
            result: EphemeralCreated {
                identify,
                reply: InitializeReply {
                    mcp_session_id,
                    result: initialize_result,
                },
            },
        }
    }

    /// EVAPORATE an ephemeral laboratory — the TERMINAL wrapper: takes
    /// the id's lifecycle lock, tears everything down, and removes the
    /// lifecycle entry (only the one it holds — `remove_if(ptr_eq)`),
    /// as the id's last act. Callers that already hold the lock
    /// mid-operation (the create fronts' stale-dupe, delete's
    /// ephemeral branch) call [`Self::evaporate_locked`] instead — the
    /// id lives on there, so the entry must stay.
    ///
    /// Callers: ephemeral SessionTerminate / Drop, host shutdown. A
    /// no-op for ids not in the registry (the freshly-minted lifecycle
    /// entry is removed again on the way out).
    async fn evaporate(self: &Arc<Self>, id: &str) {
        let (mutex, _guard) = self.lock_lifecycle(id).await;
        self.evaporate_locked(id).await;
        self.lifecycle
            .remove_if(id, |_, value| Arc::ptr_eq(value, &mutex));
    }

    /// [`Self::evaporate`], for a channel-death sweep entry: proceed
    /// only if the ephemeral is still OWNED BY `channel`. Queued
    /// behind the lifecycle mutex, a stale detach sweep could
    /// otherwise land after a successor conduit re-created the same
    /// id and evaporate the fresh container out from under it.
    async fn evaporate_if_channel(self: &Arc<Self>, id: &str, channel: u64) {
        let (mutex, _guard) = self.lock_lifecycle(id).await;
        let owned = self
            .ephemerals
            .get(id)
            .is_some_and(|lab| lab.channel == channel);
        if !owned {
            return;
        }
        self.evaporate_locked(id).await;
        self.lifecycle
            .remove_if(id, |_, value| Arc::ptr_eq(value, &mutex));
    }

    /// The evaporate body — registry removal, filetree teardown,
    /// `podman rm -f` (zero grace), `laboratory_deleted` broadcast.
    /// MUST run under the id's lifecycle lock; NEVER touches the
    /// lifecycle map (removing the entry mid-operation would let a
    /// concurrent same-id create mint a fresh mutex and run in
    /// parallel — the exact bug this split fixes). A no-op for ids not
    /// in the registry.
    async fn evaporate_locked(self: &Arc<Self>, id: &str) {
        let Some((_, _lab)) = self.ephemerals.remove(id) else {
            return;
        };
        // Tear down the Postgres tunnel proxy (plugin ephemerals): the
        // cancel wakes the accept loop and every per-connection pump,
        // which close their container sockets and remove their bridge
        // entries. Covers both evaporate paths (normal MCP-end and
        // channel-death via evaporate_if_channel) and host shutdown.
        if let Some(pg) = &_lab.pg {
            let _ = pg.cancel.send(true);
        }
        // Filetree teardown, exactly the delete_laboratory shape (see
        // its comment for why the pump abort holds attach_lock).
        {
            let _guard = self.attach_lock.lock().await;
            if let Some((_, pump)) = self.filetree_pumps.remove(id) {
                pump.abort();
            }
            self.filetree.remove(id);
        }
        self.mounts.detach_lab(id);
        self.filetree_watchers.remove(id);
        if let Err(e) = podman::laboratory::remove(&self.podman, &self.state, id).await {
            eprintln!("evaporate laboratory '{id}': {e}");
        }
        self.broadcast(&HostNotification::LaboratoryDeleted { id: id.to_string() })
            .await;
    }

    /// `PluginImageReset`: drop a development plugin's image so the
    /// next create rebuilds it.
    ///
    /// COORDINATE-level, not laboratory-level — it addresses an IMAGE,
    /// so unlike [`Self::delete_laboratory`] there is no lifecycle
    /// lock, no `LabServer` to retire and no `laboratory_deleted`
    /// broadcast. Serialization comes from the image's own bin lock,
    /// taken inside `plugin_image::reset`, which is the same one the
    /// build takes.
    async fn reset_plugin_image(
        &self,
        req: &objectiveai_sdk::laboratories::daemon::PluginImageResetRequest,
    ) -> JsonRpcResult<objectiveai_sdk::laboratories::daemon::PluginImageResetResult> {
        let coords = match crate::plugin_image::PluginCoords::canonicalize(
            &req.owner,
            &req.name,
            &req.version,
        ) {
            Ok(coords) => coords,
            Err(message) => return rpc_err(-32602, message),
        };
        match crate::plugin_image::reset(
            &self.podman,
            &self.bin_dir,
            &coords,
            req.caches,
        )
        .await
        {
            Ok(result) => JsonRpcResult::Ok { result },
            Err(message) => rpc_err(
                -32603,
                format!(
                    "reset plugin image {}/{}@{}: {message}",
                    coords.owner, coords.name, coords.version,
                ),
            ),
        }
    }

    /// `LaboratoryDelete`: retire the lab's server first (its MCP
    /// sessions die with it), force-remove the container (missing is
    /// not an error — podman's `rm -f` semantics), broadcast
    /// `laboratory_deleted`.
    ///
    /// The WHOLE body runs under the id's lifecycle lock — a Delete
    /// racing the lazy start (or an idle stop) serializes instead of
    /// yanking the container out from under an initializing
    /// `LabServer` cell. The lifecycle entry is removed as the id's
    /// last act, only-the-one-we-hold (`remove_if(ptr_eq)`).
    async fn delete_laboratory(
        self: &Arc<Self>,
        id: &str,
    ) -> JsonRpcResult<TransferAck> {
        let (mutex, _guard) = self.lock_lifecycle(id).await;
        // Defensive: a host-level Delete addressed at a live ephemeral
        // evaporates it (registry + container + broadcast) rather than
        // leaving a dangling map entry behind the rm below.
        if self.ephemerals.contains_key(id) {
            self.evaporate_locked(id).await;
            self.lifecycle
                .remove_if(id, |_, value| Arc::ptr_eq(value, &mutex));
            return JsonRpcResult::Ok {
                result: TransferAck {},
            };
        }
        self.labs.remove(id);
        // The lab's filetree watch dies with it — abort the pump and
        // drop the materialized tree (daemons clear theirs on the
        // `laboratory_deleted` broadcast below), plus the lifecycle
        // bookkeeping (watch demand and the start/stop lock). Under
        // `attach_lock`: a pump mid-`filetree_event` either finishes
        // its fold BEFORE the remove (and the entry is dropped here)
        // or blocks on the lock until after (and dies aborted at that
        // await) — without the lock it could re-create the entry via
        // `or_default`, leaving a phantom tree served to every later
        // attach. NOT held across `broadcast` below, which takes it
        // itself.
        {
            let _guard = self.attach_lock.lock().await;
            if let Some((_, pump)) = self.filetree_pumps.remove(id) {
                pump.abort();
            }
            self.filetree.remove(id);
        }
        self.mounts.detach_lab(id);
        self.filetree_watchers.remove(id);
        if let Err(e) = podman::laboratory::remove(&self.podman, &self.state, id).await {
            // The container survived; the id is still live — keep its
            // lifecycle entry.
            return rpc_err(-32603, format!("delete laboratory '{id}': {e}"));
        }
        self.broadcast(&HostNotification::LaboratoryDeleted { id: id.to_string() })
            .await;
        self.lifecycle
            .remove_if(id, |_, value| Arc::ptr_eq(value, &mutex));
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
        let source = match self.transfer_base_url(&req.source_id).await {
            Ok(base_url) => base_url,
            Err(message) => {
                return rpc_err(-32603, format!("source '{}': {message}", req.source_id));
            }
        };
        let destination = match self.transfer_base_url(&req.destination_id).await {
            Ok(base_url) => base_url,
            Err(message) => {
                return rpc_err(
                    -32603,
                    format!("destination '{}': {message}", req.destination_id),
                );
            }
        };
        match crate::transfer::pipe_export(
            &source,
            &req.source_path,
            &destination,
            &req.destination_path,
        )
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

    /// A laboratory's transfer HTTP base, whichever kind it is: a
    /// live ephemeral's recorded base URL, else the regular lab's
    /// (lazily started).
    async fn transfer_base_url(self: &Arc<Self>, id: &str) -> Result<String, String> {
        if let Some(lab) = self.ephemerals.get(id) {
            return Ok(lab.base_url.clone());
        }
        Ok(self.lab_server(id).await?.base_url().to_string())
    }

    /// Re-announce one laboratory's CURRENT identity — podman's
    /// record, notably its `running` state — to every connected
    /// daemon, as [`HostNotification::LaboratoryUpdated`]. Called on
    /// every lifecycle transition (lazy start, idle stop) so list
    /// subscribers everywhere hold live state.
    /// Build a plugin's VIEWER extension end to end (see
    /// [`crate::viewer_build`]). A plain host op — the plugin's own
    /// Containerfile is built, copied out of, and removed, so nothing
    /// long-lived exists to register as a laboratory.
    async fn build_viewer_plugin(
        &self,
        req: &objectiveai_sdk::laboratories::daemon::BuildCreateRequest,
    ) -> JsonRpcResult<objectiveai_sdk::laboratories::daemon::BuildCreated> {
        let built = crate::viewer_build::build(
            &self.podman,
            &self.bin_dir,
            &self.builds,
            &req.owner,
            &req.name,
            &req.version,
        )
        .await;
        match built {
            Ok(built) => JsonRpcResult::Ok {
                result: objectiveai_sdk::laboratories::daemon::BuildCreated {
                    commit_sha: built.commit_sha,
                    transfer_id: built.transfer_id,
                    bytes: built.bytes,
                },
            },
            // A missing git tag is the ONE build failure that is the
            // caller's rather than the plugin's — its own code, so the
            // daemon's 404 never depends on parsing prose.
            Err(crate::viewer_build::BuildFailure::TagNotFound(message)) => rpc_err(
                objectiveai_sdk::laboratories::daemon::BUILD_TAG_NOT_FOUND_CODE,
                message,
            ),
            Err(crate::viewer_build::BuildFailure::Failed(message)) => {
                rpc_err(-32603, message)
            }
        }
    }

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
        self.bridge.outbound.retain(|_, tx| {
            tx.send(crate::host_command::LaneFrame::Text(frame.clone()))
                .is_ok()
        });
    }

    /// The graceful-shutdown path. REGULAR containers this host
    /// started (initialized cells only) are STOPPED, never removed:
    /// they and their filesystems survive for the next host to
    /// `start` again. EPHEMERAL laboratories are EVAPORATED — their
    /// single connections die with this host, so the containers are
    /// garbage by definition.
    pub async fn stop_started(self: &Arc<Self>) {
        let ephemeral_ids: Vec<String> = self
            .ephemerals
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        let ids: Vec<String> = self
            .labs
            .iter()
            .filter(|entry| entry.value().get().is_some())
            .map(|entry| entry.key().clone())
            .collect();
        // ONE join across both kinds. The sets are disjoint — an id is
        // an ephemeral or a regular lab, never both — and an evaporate
        // shares nothing with a stop but podman itself, so shutdown
        // waits for the slowest container rather than the slowest
        // ephemeral plus the slowest regular lab.
        let (_, results) = tokio::join!(
            futures::future::join_all(
                ephemeral_ids.iter().map(|id| self.evaporate(id)),
            ),
            futures::future::join_all(
                ids.iter()
                    .map(|id| podman::laboratory::stop(&self.podman, &self.state, id)),
            ),
        );
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
        Req::AgentEphemeralCreate(_) => {
            Resp::AgentEphemeralCreate(rpc_err(code, message))
        }
        Req::PluginEphemeralCreate(_) => {
            Resp::PluginEphemeralCreate(rpc_err(code, message))
        }
        Req::PluginImageReset(_) => Resp::PluginImageReset(rpc_err(code, message)),
        Req::Delete(_) => Resp::Delete(rpc_err(code, message)),
        Req::LocalTransfer(_) => Resp::LocalTransfer(rpc_err(code, message)),
        Req::BuildCreate(_) => Resp::BuildCreate(rpc_err(code, message)),
        Req::BuildRead(_) => Resp::BuildRead(rpc_err(code, message)),
        Req::BuildAbort(_) => Resp::BuildAbort(rpc_err(code, message)),
    }
}

/// The caller-provided response id becomes an id suffix, a container
/// name segment, and a label field — require the bare-base62 shape the
/// API mints (non-empty, pure ASCII alphanumeric), which is safe in
/// all three positions.
fn validate_response_id(response_id: &str) -> Result<(), String> {
    if response_id.is_empty() {
        return Err("`response_id` cannot be empty".to_string());
    }
    if !response_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!(
            "`response_id` {response_id:?} must be ASCII alphanumeric",
        ));
    }
    Ok(())
}
