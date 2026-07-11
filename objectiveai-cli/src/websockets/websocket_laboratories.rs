//! The daemon's live laboratories endpoints: `/laboratories/list`
//! (every laboratory — the `laboratories list` merge as a stream) and
//! `/laboratories/{*id}` (one laboratory's full record, attachments
//! included).
//!
//! State sources, coalesced into one [`LabsChange`] feed:
//! - the CONNECTED set — [`LaboratoryRegistry`] events (a manager
//!   connected / disconnected on `/laboratory`);
//! - the LOCAL set — the machine's state-scoped container scan, via
//!   the `objectiveai-laboratory list` subprocess (the same reader the
//!   unary `laboratories list` uses; podman has no event source, so a
//!   SCANNER task polls every [`SCAN_INTERVAL`] while at least one
//!   subscriber is connected, nudged immediately on registry events
//!   and subscriber joins, with a [`SCAN_MIN_GAP`] floor);
//! - ATTACHMENTS — a dedicated `laboratory_attachments_changed`
//!   watcher with NO payload filtering (unlike `ActiveAgents`' agent
//!   watcher, which drops GROUPED-tag payloads — the per-lab record
//!   must see attachments to any tag).
//!
//! Consumers always REBUILD FROM TRUTH on a change and diff against
//! what they last sent — events carry no payloads worth trusting, and
//! a lagged subscriber self-heals on its next rebuild.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use objectiveai_sdk::cli::command::laboratories::create::{EnvVar, Mount};
use objectiveai_sdk::cli::command::laboratories::list::Source;
use objectiveai_sdk::cli::websocket_laboratories_list_listener::{
    LaboratoryEvent, LaboratoryStatus,
};
use objectiveai_sdk::cli::websocket_laboratories_listener::{
    LaboratoryAttachment, LaboratoryInstanceEvent, LaboratoryRecord,
};
use objectiveai_sdk::client_objectiveai_mcp::laboratory::Identify;
use tokio::sync::{Mutex, Notify, broadcast};

use crate::websockets::websocket_laboratory::LaboratoryRegistry;

/// Local-scan cadence while at least one subscriber is connected.
const SCAN_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
/// Floor between two scans, however hard the nudges arrive — one
/// podman subprocess per event burst, not per event.
const SCAN_MIN_GAP: std::time::Duration = std::time::Duration::from_secs(2);

/// One coalesced "something changed" tick. No payloads — every
/// consumer rebuilds from truth.
#[derive(Debug, Clone, Copy)]
pub(crate) enum LabsChange {
    /// The connected set changed (a manager connected/disconnected).
    Registry,
    /// The local scan produced a DIFFERENT set than the cache held.
    LocalScan,
    /// A `laboratory_attachments` row was written or removed
    /// (any laboratory, any target).
    Attachments,
}

/// The live-laboratories hub: the connected registry + the cached
/// local scan + the coalesced change feed, shared by both routes.
#[derive(Clone)]
pub(crate) struct LaboratoriesHub {
    registry: LaboratoryRegistry,
    ctx: crate::context::Context,
    /// Latest local (podman) scan. Refreshed by the scanner task.
    local: Arc<Mutex<Vec<Identify>>>,
    /// Live subscriber count across BOTH routes — gates the scanner
    /// (no podman subprocesses while nobody watches).
    subscribers: Arc<AtomicUsize>,
    /// Nudges the scanner out of its sleep (registry events,
    /// subscriber joins).
    rescan: Arc<Notify>,
    changes: broadcast::Sender<LabsChange>,
}

/// RAII subscriber registration: bumps the count (and nudges the
/// scanner so a fresh subscriber gets a prompt local scan), decrements
/// on drop.
pub(crate) struct SubscriberGuard {
    subscribers: Arc<AtomicUsize>,
}

impl Drop for SubscriberGuard {
    fn drop(&mut self) {
        self.subscribers.fetch_sub(1, Ordering::SeqCst);
    }
}

impl LaboratoriesHub {
    pub(crate) fn new(registry: LaboratoryRegistry, ctx: crate::context::Context) -> Self {
        Self {
            registry,
            ctx,
            local: Arc::new(Mutex::new(Vec::new())),
            subscribers: Arc::new(AtomicUsize::new(0)),
            rescan: Arc::new(Notify::new()),
            changes: broadcast::channel(1024).0,
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<LabsChange> {
        self.changes.subscribe()
    }

    fn register_subscriber(&self) -> SubscriberGuard {
        self.subscribers.fetch_add(1, Ordering::SeqCst);
        self.rescan.notify_one();
        SubscriberGuard {
            subscribers: self.subscribers.clone(),
        }
    }

    /// Spawn the hub's resident tasks: the local scanner, the
    /// registry-event forwarder, and the attachments watcher. Called
    /// once at daemon boot.
    pub(crate) fn spawn_tasks(&self) {
        tokio::spawn(self.clone().scanner());
        tokio::spawn(self.clone().forward_registry_events());
        tokio::spawn(self.clone().watch_attachment_changes());
    }

    /// The local scanner: sleeps until nudged or [`SCAN_INTERVAL`]
    /// elapses, skips entirely while nobody subscribes, floors
    /// back-to-back scans at [`SCAN_MIN_GAP`], and emits
    /// [`LabsChange::LocalScan`] only when the scan RESULT differs
    /// from the cache. Scan failures keep the previous cache (a
    /// transient podman error must not flap every local lab to
    /// removed and back).
    async fn scanner(self) {
        let mut last_scan: Option<tokio::time::Instant> = None;
        loop {
            tokio::select! {
                _ = self.rescan.notified() => {}
                _ = tokio::time::sleep(SCAN_INTERVAL) => {}
            }
            if self.subscribers.load(Ordering::SeqCst) == 0 {
                continue;
            }
            if let Some(at) = last_scan {
                let since = at.elapsed();
                if since < SCAN_MIN_GAP {
                    tokio::time::sleep(SCAN_MIN_GAP - since).await;
                }
            }
            last_scan = Some(tokio::time::Instant::now());
            let Ok(fresh) = local_scan(&self.ctx).await else {
                continue;
            };
            let changed = {
                let mut cache = self.local.lock().await;
                if *cache == fresh {
                    false
                } else {
                    *cache = fresh;
                    true
                }
            };
            if changed {
                let _ = self.changes.send(LabsChange::LocalScan);
            }
        }
    }

    /// Forward registry connect/disconnect events into the coalesced
    /// feed and nudge the scanner (a connect often accompanies a
    /// just-created local container).
    async fn forward_registry_events(self) {
        let mut rx = self.registry.subscribe();
        loop {
            match rx.recv().await {
                Ok(_) => {
                    let _ = self.changes.send(LabsChange::Registry);
                    self.rescan.notify_one();
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Coalesced anyway — one Registry tick covers any
                    // number of missed events.
                    let _ = self.changes.send(LabsChange::Registry);
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }

    /// Subscribe to the `laboratory_attachments_changed` NOTIFY
    /// channel and emit [`LabsChange::Attachments`] for EVERY
    /// notification — no payload filtering (the payload names the
    /// agent target, not the laboratory; consumers re-query by lab
    /// id). Same reconnect loop as `ActiveAgents`' watchers.
    async fn watch_attachment_changes(self) {
        use std::time::Duration;
        loop {
            let reconnect = async {
                let pool = self.ctx.db_client().await.ok()?;
                let mut listener =
                    sqlx::postgres::PgListener::connect_with(&**pool).await.ok()?;
                listener.listen("laboratory_attachments_changed").await.ok()?;
                Some(listener)
            }
            .await;
            let Some(mut listener) = reconnect else {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            };
            while listener.recv().await.is_ok() {
                let _ = self.changes.send(LabsChange::Attachments);
            }
            // Listener errored/closed — pause, then reconnect.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// The full merged list — connected ∪ local scan, classified by
    /// RAW id exactly like the unary `laboratories list` (connected
    /// labs first: `source` local when the scan knows the id, remote
    /// otherwise; then local labs with no live connection).
    async fn merged_list(&self) -> Vec<LaboratoryStatus> {
        let connected = self.registry.list();
        let local = self.local.lock().await.clone();
        let local_ids: HashSet<&str> = local.iter().map(|l| l.id.as_str()).collect();
        let connected_ids: HashSet<String> =
            connected.iter().map(|l| l.id.clone()).collect();
        let mut out = Vec::with_capacity(connected.len() + local.len());
        for lab in connected {
            let source = if local_ids.contains(lab.id.as_str()) {
                Source::Local
            } else {
                Source::Remote
            };
            out.push(status_from_identify(lab, source, true));
        }
        for lab in local {
            if !connected_ids.contains(&lab.id) {
                out.push(status_from_identify(lab, Source::Local, false));
            }
        }
        out
    }

    /// One laboratory's full record: identity from the registry
    /// (connected) or the local cache, zero-filled when absent from
    /// both (`source: None` — attachment rows can outlive their
    /// laboratory); attachments from the DB. `None` when the DB is
    /// unavailable (the frame is skipped; a later change retries).
    async fn build_record(&self, id: &str) -> Option<LaboratoryRecord> {
        let connected_identity =
            self.registry.list().into_iter().find(|lab| lab.id == id);
        let connected = connected_identity.is_some();
        let local_identity = self
            .local
            .lock()
            .await
            .iter()
            .find(|lab| lab.id == id)
            .cloned();
        let locally_present = local_identity.is_some();
        let source = if locally_present {
            Some(Source::Local)
        } else if connected {
            Some(Source::Remote)
        } else {
            None
        };
        let identity = connected_identity.or(local_identity);

        let pool = self.ctx.db_client().await.ok()?;
        let rows =
            crate::db::laboratory_attachments::list_for_laboratory(pool, id)
                .await
                .ok()?;
        let attachments = rows
            .into_iter()
            .filter_map(|row| match (row.agent_instance_hierarchy, row.tag) {
                (Some(agent_instance_hierarchy), _) => Some(LaboratoryAttachment::Aih {
                    agent_instance_hierarchy,
                    attached_at: row.attached_at,
                    attached_by: row.attached_by,
                }),
                (None, Some(tag)) => Some(LaboratoryAttachment::Tag {
                    tag,
                    attached_at: row.attached_at,
                    attached_by: row.attached_by,
                }),
                // Both NULL is unrepresentable (table CHECK); skip
                // defensively rather than invent a target.
                (None, None) => None,
            })
            .collect();

        let (image, mounts, env, cwd) = match identity {
            Some(identify) => (
                Some(identify.image),
                identify
                    .mounts
                    .into_iter()
                    .map(|m| Mount {
                        host: m.host,
                        container: m.container,
                    })
                    .collect(),
                identify
                    .env
                    .into_iter()
                    .map(|[key, value]| EnvVar { key, value })
                    .collect(),
                Some(identify.cwd),
            ),
            None => (None, Vec::new(), Vec::new(), None),
        };
        Some(LaboratoryRecord {
            id: id.to_string(),
            image,
            mounts,
            env,
            cwd,
            source,
            connected,
            attachments,
        })
    }
}

/// [`Identify`] → one list item.
fn status_from_identify(
    lab: Identify,
    source: Source,
    connected: bool,
) -> LaboratoryStatus {
    LaboratoryStatus {
        id: lab.id,
        image: lab.image,
        mounts: lab
            .mounts
            .into_iter()
            .map(|m| Mount {
                host: m.host,
                container: m.container,
            })
            .collect(),
        env: lab
            .env
            .into_iter()
            .map(|[key, value]| EnvVar { key, value })
            .collect(),
        cwd: lab.cwd,
        source,
        connected,
    }
}

/// The local machine's laboratories via the manager binary's `list`
/// subcommand — the same reader as the unary `laboratories list`
/// (`command::laboratories::list::local_laboratories`); a missing
/// binary is an empty set (remote-only install).
async fn local_scan(
    ctx: &crate::context::Context,
) -> Result<Vec<Identify>, ()> {
    let exe = ctx.filesystem.bin_dir().join(if cfg!(windows) {
        "objectiveai-laboratory.exe"
    } else {
        "objectiveai-laboratory"
    });
    let output = match tokio::process::Command::new(&exe)
        .arg("list")
        .arg("--objectiveai-dir")
        .arg(ctx.filesystem.dir())
        .arg("--objectiveai-state")
        .arg(ctx.filesystem.state())
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(()),
    };
    if !output.status.success() {
        return Err(());
    }
    serde_json::from_slice(&output.stdout).map_err(|_| ())
}

// ── the two routes ──────────────────────────────────────────────────

/// `/laboratories/list`: upgrade, consume the auth preamble, send the
/// snapshot, then stream `Upserted`/`Removed` deltas.
pub(crate) async fn laboratories_handler(
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
        laboratories_list_pump(socket, state.labs_hub).await;
    })
}

/// Send the connect snapshot, then rebuild-and-diff on every change,
/// emitting per-lab deltas. Subscribes BEFORE the snapshot so no
/// change slips the gap; `Lagged` self-heals (the next rebuild+diff
/// covers everything missed).
async fn laboratories_list_pump(
    mut socket: axum::extract::ws::WebSocket,
    hub: LaboratoriesHub,
) {
    use axum::extract::ws::Message;
    let mut rx = hub.subscribe();
    let _guard = hub.register_subscriber();

    let mut last: BTreeMap<String, LaboratoryStatus> = hub
        .merged_list()
        .await
        .into_iter()
        .map(|lab| (lab.id.clone(), lab))
        .collect();
    let snapshot = LaboratoryEvent::Snapshot {
        laboratories: last.values().cloned().collect(),
    };
    if let Ok(frame) = serde_json::to_string(&snapshot) {
        if socket.send(Message::Text(frame.into())).await.is_err() {
            return;
        }
    }
    loop {
        tokio::select! {
            received = rx.recv() => {
                match received {
                    // Attachment changes don't ride the list; identity
                    // and connected-ness are its whole payload.
                    Ok(LabsChange::Attachments) => continue,
                    Ok(LabsChange::Registry | LabsChange::LocalScan)
                    | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
                let next: BTreeMap<String, LaboratoryStatus> = hub
                    .merged_list()
                    .await
                    .into_iter()
                    .map(|lab| (lab.id.clone(), lab))
                    .collect();
                let mut frames: Vec<LaboratoryEvent> = Vec::new();
                for (id, lab) in &next {
                    if last.get(id) != Some(lab) {
                        frames.push(LaboratoryEvent::Upserted {
                            laboratory: lab.clone(),
                        });
                    }
                }
                for id in last.keys() {
                    if !next.contains_key(id) {
                        frames.push(LaboratoryEvent::Removed { id: id.clone() });
                    }
                }
                last = next;
                for event in frames {
                    let Ok(frame) = serde_json::to_string(&event) else {
                        continue;
                    };
                    if socket.send(Message::Text(frame.into())).await.is_err() {
                        return;
                    }
                }
            }
            inbound = socket.recv() => match inbound {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => {}
            },
        }
    }
}

/// `/laboratories/{*id}`: upgrade, consume the auth preamble, send
/// the record, then re-send it (full-value) on every relevant change.
pub(crate) async fn laboratory_instance_handler(
    axum::extract::State(state): axum::extract::State<
        crate::websockets::daemon_stream::DaemonWsState,
    >,
    axum::extract::Path(id): axum::extract::Path<String>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        if !crate::websockets::daemon_auth::authenticate(&mut socket, state.secret.as_ref())
            .await
        {
            return;
        }
        laboratory_instance_pump(socket, state.labs_hub, id).await;
    })
}

/// Send the current record, then rebuild on EVERY change (registry,
/// local scan, attachments — all three can alter it) and send only
/// when the record actually differs. A DB outage skips the frame; the
/// next change retries.
async fn laboratory_instance_pump(
    mut socket: axum::extract::ws::WebSocket,
    hub: LaboratoriesHub,
    id: String,
) {
    use axum::extract::ws::Message;
    let mut rx = hub.subscribe();
    let _guard = hub.register_subscriber();

    let mut last: Option<LaboratoryRecord> = hub.build_record(&id).await;
    if let Some(record) = &last {
        let event = LaboratoryInstanceEvent::Laboratory {
            laboratory: record.clone(),
        };
        if let Ok(frame) = serde_json::to_string(&event) {
            if socket.send(Message::Text(frame.into())).await.is_err() {
                return;
            }
        }
    }
    loop {
        tokio::select! {
            received = rx.recv() => {
                match received {
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
                let Some(record) = hub.build_record(&id).await else {
                    continue;
                };
                if last.as_ref() == Some(&record) {
                    continue;
                }
                let event = LaboratoryInstanceEvent::Laboratory {
                    laboratory: record.clone(),
                };
                last = Some(record);
                let Ok(frame) = serde_json::to_string(&event) else {
                    continue;
                };
                if socket.send(Message::Text(frame.into())).await.is_err() {
                    break;
                }
            }
            inbound = socket.recv() => match inbound {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => {}
            },
        }
    }
}
