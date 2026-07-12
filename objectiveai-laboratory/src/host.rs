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

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{
    ChannelRequest, ChannelResponse, HostIdentify, HostNotification, Identify, IdentifyMount,
};
use objectiveai_sdk::client_objectiveai_mcp::server_response::JsonRpcResult;
use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};
use objectiveai_sdk::machine::MachineIdentity;

use crate::podman;
use crate::server::LabServer;

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
        let id = self.next_outbound.fetch_add(1, Ordering::Relaxed);
        self.outbound.insert(id, reply_tx);
        id
    }

    pub fn detach_channel(&self, id: u64) {
        self.outbound.remove(&id);
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

    /// Serve one request from any daemon channel; the reply echoes the
    /// correlation id. Host-level ops (create/delete) run here;
    /// everything else demuxes by `laboratory_id` to a lazily-started
    /// [`LabServer`].
    pub async fn handle(self: &Arc<Self>, request: ChannelRequest) -> ChannelResponse {
        match &request.payload {
            server_request::Payload::LaboratoryCreate(req) => {
                let result = self.create_laboratory(req).await;
                return ChannelResponse {
                    id: request.id,
                    payload: server_response::Payload::LaboratoryCreate(result),
                };
            }
            server_request::Payload::LaboratoryDelete(req) => {
                let result = self.delete_laboratory(&req.id).await;
                return ChannelResponse {
                    id: request.id,
                    payload: server_response::Payload::LaboratoryDelete(result),
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
        match self.lab_server(&lab_id).await {
            Ok(server) => server.handle(request).await,
            Err(message) => ChannelResponse {
                payload: reject(&request.payload, -32603, message),
                id: request.id,
            },
        }
    }

    /// The laboratory's server, starting its container on first use.
    async fn lab_server(&self, id: &str) -> Result<Arc<LabServer>, String> {
        let cell = self
            .labs
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::OnceCell::new()))
            .clone();
        cell.get_or_try_init(|| async {
            // Start-not-create: a stopped container resumes with its
            // filesystem intact.
            podman::laboratory::start(&self.podman, &self.state, id)
                .await
                .map_err(|e| format!("start laboratory '{id}': {e}"))?;
            match podman::laboratory::host_port(&self.podman, &self.state, id).await {
                Ok(port) => Ok(Arc::new(LabServer::new(format!("http://127.0.0.1:{port}")))),
                Err(e) => {
                    // We just started it — don't leak a running
                    // container behind a failed init.
                    let _ = podman::laboratory::stop(&self.podman, &self.state, id).await;
                    Err(format!("laboratory '{id}' port: {e}"))
                }
            }
        })
        .await
        .cloned()
    }

    /// `LaboratoryCreate`: podman create + MCP binary injection
    /// (container NOT started), echo the created spec, broadcast
    /// `laboratory_created` to every connected daemon.
    async fn create_laboratory(
        &self,
        req: &server_request::LaboratoryCreateRequest,
    ) -> JsonRpcResult<Identify> {
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
    ) -> JsonRpcResult<server_response::LaboratoryTransferAck> {
        self.labs.remove(id);
        if let Err(e) = podman::laboratory::remove(&self.podman, &self.state, id).await {
            return rpc_err(-32603, format!("delete laboratory '{id}': {e}"));
        }
        self.broadcast(&HostNotification::LaboratoryDeleted { id: id.to_string() })
            .await;
        JsonRpcResult::Ok {
            result: server_response::LaboratoryTransferAck {},
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
    payload: &server_request::Payload,
    code: i64,
    message: String,
) -> server_response::Payload {
    use server_request::Payload as Req;
    use server_response::Payload as Resp;
    match payload {
        Req::Initialize { mcp_kind, .. } => Resp::Initialize {
            mcp_kind: mcp_kind.clone(),
            result: rpc_err(code, message),
        },
        Req::ToolsList { mcp_kind, .. } => Resp::ToolsList {
            mcp_kind: mcp_kind.clone(),
            result: rpc_err(code, message),
        },
        Req::ToolsCall { mcp_kind, .. } => Resp::ToolsCall {
            mcp_kind: mcp_kind.clone(),
            result: rpc_err(code, message),
        },
        Req::ResourcesList { mcp_kind, .. } => Resp::ResourcesList {
            mcp_kind: mcp_kind.clone(),
            result: rpc_err(code, message),
        },
        Req::ResourcesRead { mcp_kind, .. } => Resp::ResourcesRead {
            mcp_kind: mcp_kind.clone(),
            result: rpc_err(code, message),
        },
        Req::SessionTerminate { mcp_kind } => Resp::SessionTerminate {
            mcp_kind: mcp_kind.clone(),
            result: rpc_err(code, message),
        },
        Req::ReadMessageQueue(_) => Resp::ReadMessageQueue(rpc_err(code, message)),
        Req::Retrieve(_) => Resp::Retrieve(rpc_err(code, message)),
        Req::Drop(_) => Resp::Drop(server_response::DropResult { dropped: false }),
        Req::LaboratoryExportBegin(_) => Resp::LaboratoryExportBegin(rpc_err(code, message)),
        Req::LaboratoryExportRead(_) => Resp::LaboratoryExportRead(rpc_err(code, message)),
        Req::LaboratoryExportAbort(_) => Resp::LaboratoryExportAbort(rpc_err(code, message)),
        Req::LaboratoryImportBegin(_) => Resp::LaboratoryImportBegin(rpc_err(code, message)),
        Req::LaboratoryImportWrite(_) => Resp::LaboratoryImportWrite(rpc_err(code, message)),
        Req::LaboratoryImportEnd(_) => Resp::LaboratoryImportEnd(rpc_err(code, message)),
        Req::LaboratoryImportAbort(_) => Resp::LaboratoryImportAbort(rpc_err(code, message)),
        // Host-level ops are answered in `handle` — reaching here is a
        // routing bug, but the reply shape still pairs correctly.
        Req::LaboratoryCreate(_) => Resp::LaboratoryCreate(rpc_err(code, message)),
        Req::LaboratoryDelete(_) => Resp::LaboratoryDelete(rpc_err(code, message)),
    }
}
