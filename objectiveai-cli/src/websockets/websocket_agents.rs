//! The resident daemon's live agent-status hub — the `/agents/instances/list` endpoint.
//!
//! Robust active/inactive tracking, driven by the per-agent lockfile
//! (`objectiveai_sdk::lockfile`) rather than by stream lifecycles:
//!
//! - **Producer side** — a fixed-name local socket (`<state>/socks/agents.sock`
//!   on Unix; a namespaced pipe on Windows), SEPARATE from `daemon.sock`.
//!   Every place the CLI acquires an `agent_instance_hierarchy` (AIH)
//!   instance lock (via [`crate::websockets::agent_registry`]) fires a
//!   one-line [`ActiveAnnounce`] over this socket: "AIH X is now active."
//! - **Watcher** — on each announce the daemon spawns
//!   [`objectiveai_sdk::lockfile::wait_released`] for that AIH's instance
//!   lock. The kernel releases a `flock`/`LockFileEx` even when its holder
//!   is killed, so a spawn killed mid-stream flips to inactive exactly —
//!   no leak, no reliance on a clean stream end.
//! - **Consumer side** — the [`axum`] WebSocket `/agents/instances/list` route
//!   (registered by [`crate::websockets::daemon_stream::serve_ws`]). On
//!   connect a client gets one [`AgentEvent::Snapshot`] of ALL agents
//!   (from the DB), then streams [`AgentEvent::Activated`] /
//!   [`AgentEvent::Deactivated`] deltas.
//!
//! `last_active_at` is stamped ONLY on the active→inactive flip: while an
//! agent is active its last-active is implicitly "now", so it rides the
//! wire as `None` and is filled at the moment its lock releases.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{ListenerOptions, Name};
use objectiveai_sdk::cli::command::agents::instances::list::ResponseItem;
use objectiveai_sdk::cli::websocket_agents_listener::{AgentEvent, AgentRecord};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, broadcast};

use crate::websockets::mcp_listener::socks_dir;

/// One-line producer message: "this AIH just acquired its instance lock."
/// CLI-internal (not part of the codegen'd wire surface); the AIH is the
/// whole payload — the daemon derives everything else from the DB + the
/// lock's release.
#[derive(serde::Serialize, serde::Deserialize)]
struct ActiveAnnounce {
    agent_instance_hierarchy: String,
}

/// The fixed local-socket name for the agents hub, identical on the
/// listener and producer sides. Mirrors
/// [`crate::websockets::daemon_stream`]'s scheme with the constant
/// `agents` in place of `daemon`.
#[cfg(unix)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    socks_dir(state_dir)
        .join("agents.sock")
        .to_fs_name::<GenericFilePath>()
}

#[cfg(windows)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    use std::hash::{Hash, Hasher};
    // Named pipes are machine-global; fold the state NAME into the pipe
    // name to preserve the per-state isolation the Unix `<state>/socks/`
    // path gives (matching `daemon_stream`/`mcp_listener`).
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    state_dir.file_name().hash(&mut hasher);
    let state = hasher.finish();
    format!("objectiveai-{state:016x}-agents.sock").to_ns_name::<GenericNamespaced>()
}

/// Bind the fixed-name agents producer socket, returning the bound
/// listener. Bound **synchronously** under the daemon init gate (like
/// [`crate::websockets::daemon_stream::bind_socket_listener`]) so a held
/// daemon lock guarantees the socket is up. `try_overwrite` clears a stale
/// socket file left by a crashed predecessor.
pub fn bind_agents_socket_listener(
    state_dir: &Path,
) -> std::io::Result<interprocess::local_socket::tokio::Listener> {
    let _ = std::fs::create_dir_all(socks_dir(state_dir));
    let name = socket_name(state_dir)?;
    ListenerOptions::new()
        .name(name)
        .try_overwrite(true)
        .create_tokio()
}

/// Shared live-agent registry + delta broadcast. Cloned into the WS state
/// and the socket accept loop; the sender clones keep the broadcast open
/// for the daemon's whole life.
#[derive(Clone)]
pub(crate) struct ActiveAgents {
    /// The set of AIHs whose instance lock is currently held. A
    /// `tokio::sync::Mutex` so the release watcher can re-probe held-state
    /// under the lock, serializing correctly against concurrent
    /// [`activate`](Self::activate) (no lost activation on fast reacquire).
    active: Arc<Mutex<HashSet<String>>>,
    /// Pre-serialized [`AgentEvent`] JSON frames, fanned to every `/agents/instances/list`
    /// subscriber.
    events: broadcast::Sender<String>,
    state_dir: PathBuf,
    /// Resident context — the DB pool is resolved lazily (`db_client`), as
    /// the daemon boots before a DB necessarily exists.
    ctx: crate::context::Context,
}

impl ActiveAgents {
    pub(crate) fn new(
        state_dir: PathBuf,
        events: broadcast::Sender<String>,
        ctx: crate::context::Context,
    ) -> Self {
        Self {
            active: Arc::new(Mutex::new(HashSet::new())),
            events,
            state_dir,
            ctx,
        }
    }

    /// A fresh subscription to the delta stream.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<String> {
        self.events.subscribe()
    }

    /// Serialize + fan one event out. A send error means no `/agents/instances/list`
    /// clients are connected — drop the frame.
    fn broadcast(&self, event: &AgentEvent) {
        if let Ok(frame) = serde_json::to_string(event) {
            let _ = self.events.send(frame);
        }
    }

    /// Mark `aih` active. Idempotent: if it is already active (a reentrant
    /// parent→child lock transfer re-announces the same AIH), this is a
    /// no-op — one watcher, one `Activated`, per active lifetime. Otherwise
    /// it records the AIH, broadcasts `Activated`, and spawns the release
    /// watcher.
    pub(crate) async fn activate(&self, aih: String) {
        {
            let mut active = self.active.lock().await;
            if !active.insert(aih.clone()) {
                return;
            }
        }
        let agent = self.build_active_record(&aih).await;
        self.broadcast(&AgentEvent::Activated { agent });
        let this = self.clone();
        tokio::spawn(async move { this.watch(aih).await });
    }

    /// Build the `Activated` record for `aih`, preferring DB truth
    /// ([`crate::db::instances::get_exact`]); on a brand-new agent (no
    /// `messages` row yet) or DB-unavailable, a minimal `active` record.
    async fn build_active_record(&self, aih: &str) -> AgentRecord {
        if let Ok(pool) = self.ctx.db_client().await {
            if let Ok(item) = crate::db::instances::get_exact(pool, aih).await {
                return record_from_item(&item, true);
            }
        }
        AgentRecord {
            agent_instance_hierarchy: aih.to_string(),
            tags: Vec::new(),
            queued: 0,
            logged: 0,
            active: true,
            spawned_at: None,
            last_active_at: None,
        }
    }

    /// Watch `aih`'s instance lock until it is released (or its holder
    /// dies), then flip it inactive. Re-probes held-state under the map
    /// lock so a reacquire during the wake gap keeps the AIH active with no
    /// spurious delta.
    async fn watch(self, aih: String) {
        let (dir, key) =
            crate::command::agents::locks::agent_instance_lock(&self.state_dir, &aih);
        loop {
            // Wakes on release OR holder death (kernel drops the flock /
            // LockFileEx). An error is treated as "released" via the probe.
            let _ = objectiveai_sdk::lockfile::wait_released(&dir, &key).await;
            let mut active = self.active.lock().await;
            // A new holder may have acquired during the wake gap (fast
            // reacquire / transfer). Under the lock so `activate` cannot
            // interleave and lose the transition.
            if objectiveai_sdk::lockfile::try_held(&dir, &key).await {
                drop(active);
                continue;
            }
            active.remove(&aih);
            drop(active);
            let last =
                crate::db::time::unix_to_rfc3339(chrono::Utc::now().timestamp());
            self.broadcast(&AgentEvent::Deactivated {
                agent_instance_hierarchy: aih,
                last_active_at: Some(last),
            });
            break;
        }
    }

    /// The connect-time snapshot: ALL agents from the DB, each with its
    /// `active` flag from the registry, plus a minimal record for any
    /// active AIH not yet in the DB (brand-new).
    async fn snapshot(&self) -> Vec<AgentRecord> {
        let items = match self.ctx.db_client().await {
            Ok(pool) => crate::db::instances::list_all(pool).await.unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let active = self.active.lock().await;
        let mut out = Vec::with_capacity(items.len());
        let mut seen: HashSet<&str> = HashSet::new();
        for item in &items {
            let is_active = active.contains(&item.agent_instance_hierarchy);
            seen.insert(item.agent_instance_hierarchy.as_str());
            out.push(record_from_item(item, is_active));
        }
        for aih in active.iter() {
            if !seen.contains(aih.as_str()) {
                out.push(AgentRecord {
                    agent_instance_hierarchy: aih.clone(),
                    tags: Vec::new(),
                    queued: 0,
                    logged: 0,
                    active: true,
                    spawned_at: None,
                    last_active_at: None,
                });
            }
        }
        out
    }

    /// Build the current record for `aih` — DB truth
    /// ([`crate::db::instances::get_exact`]) with the `active` flag from
    /// the registry (a live agent's `last_active_at` is suppressed).
    /// `None` if the DB is unavailable.
    async fn build_record_for(&self, aih: &str) -> Option<AgentRecord> {
        let active = self.active.lock().await.contains(aih);
        let pool = self.ctx.db_client().await.ok()?;
        let item = crate::db::instances::get_exact(pool, aih).await.ok()?;
        Some(record_from_item(&item, active))
    }

    /// Subscribe to the `tags_changed` NOTIFY channel and broadcast an
    /// [`AgentEvent::Updated`] for each AIH whose bound tags changed (tag
    /// applied / moved / removed — a trigger on `objectiveai.tags` fires
    /// the AIH as payload). Runs for the daemon's life; on a listener
    /// error it reconnects after a short pause. This is the persisted-state
    /// counterpart to the lock-driven active/inactive tracking: tags live
    /// in the DB, so the DB is the authoritative change signal.
    pub(crate) async fn watch_tag_changes(self) {
        use std::time::Duration;
        loop {
            let reconnect = async {
                let pool = self.ctx.db_client().await.ok()?;
                let mut listener =
                    sqlx::postgres::PgListener::connect_with(&**pool).await.ok()?;
                listener.listen("tags_changed").await.ok()?;
                Some(listener)
            }
            .await;
            let Some(mut listener) = reconnect else {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            };
            while let Ok(notification) = listener.recv().await {
                let aih = notification.payload().to_string();
                if let Some(agent) = self.build_record_for(&aih).await {
                    self.broadcast(&AgentEvent::Updated { agent });
                }
            }
            // Listener errored/closed — pause, then reconnect.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Best-effort startup reconcile: seed the registry with agents already
    /// holding an instance lock when the daemon starts (or before any
    /// client connects). Probes `try_held` per candidate AIH from
    /// `list_all` (`owners_in_tree` returns PIDs, not AIHs, so it cannot
    /// reconstruct the hierarchy). Errors are ignored.
    pub(crate) async fn reconcile_startup(&self) {
        let Ok(pool) = self.ctx.db_client().await else {
            return;
        };
        let Ok(items) = crate::db::instances::list_all(pool).await else {
            return;
        };
        for item in items {
            let (dir, key) = crate::command::agents::locks::agent_instance_lock(
                &self.state_dir,
                &item.agent_instance_hierarchy,
            );
            if objectiveai_sdk::lockfile::try_held(&dir, &key).await {
                self.activate(item.agent_instance_hierarchy).await;
            }
        }
    }
}

/// Map an `agents instances list` item to an [`AgentRecord`]. `created_at`
/// becomes `spawned_at`; a live agent's `last_active_at` is suppressed
/// (implicitly "now").
fn record_from_item(item: &ResponseItem, active: bool) -> AgentRecord {
    AgentRecord {
        agent_instance_hierarchy: item.agent_instance_hierarchy.clone(),
        tags: item.tags.clone(),
        queued: item.queued,
        logged: item.logged,
        active,
        spawned_at: item.created_at.clone(),
        last_active_at: if active {
            None
        } else {
            item.last_active_at.clone()
        },
    }
}

/// Spawn the accept loop on the pre-bound agents socket: one task per
/// connection, each reading a single [`ActiveAnnounce`] line and marking
/// the AIH active.
pub fn serve_agents_socket_listener(
    listener: interprocess::local_socket::tokio::Listener,
    active: ActiveAgents,
) {
    tokio::spawn(async move {
        loop {
            let conn = match listener.accept().await {
                Ok(conn) => conn,
                // Transient accept error — keep serving.
                Err(_) => continue,
            };
            let active = active.clone();
            tokio::spawn(async move {
                let (read_half, _write_half) = tokio::io::split(conn);
                let mut reader = BufReader::new(read_half);
                let mut line = String::new();
                if reader.read_line(&mut line).await.is_err() {
                    return;
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return;
                }
                if let Ok(announce) = serde_json::from_str::<ActiveAnnounce>(trimmed) {
                    active.activate(announce.agent_instance_hierarchy).await;
                }
            });
        }
    });
}

/// Producer helper: announce "AIH just went active" to the daemon's agents
/// socket. Best-effort — a missing/dead daemon socket is a silent no-op
/// (the agent runs regardless); idempotent (the daemon dedupes by AIH, so
/// callers need not track prior announcements). The single retried error
/// is Windows `ERROR_PIPE_BUSY` (a live listener mid-accept), same as
/// [`crate::websockets::daemon_stream::connect_feed`].
pub async fn announce_active(state_dir: &Path, aih: &str) {
    let announce = ActiveAnnounce {
        agent_instance_hierarchy: aih.to_string(),
    };
    let Ok(line) = serde_json::to_string(&announce) else {
        return;
    };
    const ERROR_PIPE_BUSY: i32 = 231;
    let mut attempts = 0u32;
    let conn = loop {
        let Ok(name) = socket_name(state_dir) else {
            return;
        };
        match LocalSocketStream::connect(name).await {
            Ok(conn) => break conn,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && attempts < 20 => {
                attempts += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
            // Daemon not running / socket absent → best-effort no-op.
            Err(_) => return,
        }
    };
    let (_read_half, mut write_half) = tokio::io::split(conn);
    let _ = write_half.write_all(line.as_bytes()).await;
    let _ = write_half.write_all(b"\n").await;
    let _ = write_half.flush().await;
    let _ = write_half.shutdown().await;
}

/// `/agents/instances/list`: upgrade to WebSocket, consume the auth preamble, send the
/// snapshot, then stream deltas.
pub(crate) async fn agents_handler(
    axum::extract::State(state): axum::extract::State<
        crate::websockets::daemon_stream::DaemonWsState,
    >,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        if !crate::websockets::daemon_auth::authenticate(&mut socket, state.secret.as_ref())
            .await
        {
            return;
        }
        agents_pump(socket, state.active).await;
    })
}

/// Send the connect snapshot, then forward every delta frame until the
/// client disconnects. Subscribes BEFORE building the snapshot so no delta
/// slips through the gap; a client may thus see one delta already folded
/// into the snapshot — consumers key by AIH. `Lagged` (slow client) drops
/// missed deltas and keeps going, like `daemon_stream::pump`.
async fn agents_pump(mut socket: axum::extract::ws::WebSocket, active: ActiveAgents) {
    use axum::extract::ws::Message;
    let mut rx = active.subscribe();
    let snapshot = AgentEvent::Snapshot {
        agents: active.snapshot().await,
    };
    if let Ok(frame) = serde_json::to_string(&snapshot) {
        if socket.send(Message::Text(frame.into())).await.is_err() {
            return;
        }
    }
    loop {
        tokio::select! {
            received = rx.recv() => match received {
                Ok(frame) => {
                    if socket.send(Message::Text(frame.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            inbound = socket.recv() => match inbound {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => {}
            },
        }
    }
}
