//! The daemon's live laboratories endpoints: `/laboratories/list`
//! (every laboratory served by a connected host, as a stream) and
//! `/laboratories/{*id}` (one laboratory's full record, attachments
//! included).
//!
//! State sources, coalesced into one [`LabsChange`] feed:
//! - the REGISTRY — [`LaboratoryRegistry`] events (a host connected or
//!   disconnected, a laboratory was created or deleted on a connected
//!   host). The registry IS the laboratory set: hosts announce their
//!   full list on connect and notify on every change, so there is no
//!   scan, no poll, and no local-vs-remote split — machine identity is
//!   the only provenance;
//! - ATTACHMENTS — a dedicated `laboratory_attachments_changed`
//!   watcher with NO payload filtering (unlike `ActiveAgents`' agent
//!   watcher, which drops GROUPED-tag payloads — the per-lab record
//!   must see attachments to any tag).
//!
//! Consumers always REBUILD FROM TRUTH on a change and diff against
//! what they last sent — events carry no payloads worth trusting, and
//! a lagged subscriber self-heals on its next rebuild.

use std::collections::BTreeMap;

use objectiveai_sdk::cli::command::laboratories::create::{EnvVar, Mount};
use objectiveai_sdk::cli::websocket_laboratories_list_listener::{
    LaboratoryEvent, LaboratoryStatus,
};
use objectiveai_sdk::cli::websocket_laboratories_listener::{
    LaboratoryAttachment, LaboratoryInstanceEvent, LaboratoryRecord,
};
use objectiveai_sdk::laboratories::daemon::Identify;
use objectiveai_sdk::machine::MachineIdentity;
use tokio::sync::broadcast;

use crate::websockets::websocket_laboratory::LaboratoryRegistry;

/// One coalesced "something changed" tick. No payloads — every
/// consumer rebuilds from truth.
#[derive(Debug, Clone, Copy)]
pub(crate) enum LabsChange {
    /// The registry changed (a host connected/disconnected, or a
    /// laboratory was created/deleted on a connected host).
    Registry,
    /// A `laboratory_attachments` row was written or removed
    /// (any laboratory, any target).
    Attachments,
}

/// The live-laboratories hub: the host registry + the coalesced change
/// feed, shared by both routes.
#[derive(Clone)]
pub(crate) struct LaboratoriesHub {
    registry: LaboratoryRegistry,
    ctx: crate::context::Context,
    changes: broadcast::Sender<LabsChange>,
}

impl LaboratoriesHub {
    pub(crate) fn new(registry: LaboratoryRegistry, ctx: crate::context::Context) -> Self {
        Self {
            registry,
            ctx,
            changes: broadcast::channel(1024).0,
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<LabsChange> {
        self.changes.subscribe()
    }

    /// Spawn the hub's resident tasks: the registry-event forwarder and
    /// the attachments watcher. Called once at daemon boot.
    pub(crate) fn spawn_tasks(&self) {
        tokio::spawn(self.clone().forward_registry_events());
        tokio::spawn(self.clone().watch_attachment_changes());
    }

    /// Forward registry events into the coalesced change feed (which
    /// drives the pumps to rebuild).
    async fn forward_registry_events(self) {
        let mut rx = self.registry.subscribe();
        loop {
            match rx.recv().await {
                Ok(_) => {
                    let _ = self.changes.send(LabsChange::Registry);
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

    /// The full list — the registry snapshot, one item per laboratory
    /// served by a connected host. Everything listed is connected by
    /// construction (the registry IS the live connections). Same-id
    /// items from different hosts coexist — laboratory ids are only
    /// unique per (machine, state).
    async fn list(&self) -> Vec<LaboratoryStatus> {
        self.registry
            .list()
            .await
            .into_iter()
            .map(|(machine, state, identify)| status_from_identify(identify, machine, state))
            .collect()
    }

    /// One laboratory's full record: identity + machine from the
    /// registry when a connected host serves it, zero-filled otherwise
    /// (`machine: None` — attachment rows can outlive their
    /// laboratory); attachments from the DB. `host` (the route's
    /// optional `?machine=…&machine_state=…`) pins the exact
    /// laboratory — identity resolution AND the attachment rows are
    /// narrowed to it; without it, first-match-by-id and every row
    /// with the id (legacy behavior). `None` when the DB is
    /// unavailable (the frame is skipped; a later change retries).
    async fn build_record(
        &self,
        id: &str,
        host: Option<(&str, &str)>,
    ) -> Option<LaboratoryRecord> {
        let identity = self
            .registry
            .list()
            .await
            .into_iter()
            .find(|(machine, state, lab)| {
                lab.id == id
                    && host.is_none_or(|(host_machine, host_state)| {
                        machine.id == host_machine && state == host_state
                    })
            });
        let connected = identity.is_some();

        let pool = self.ctx.db_client().await.ok()?;
        let rows =
            crate::db::laboratory_attachments::list_for_laboratory(pool, id, host)
                .await
                .ok()?;
        let attachments = rows
            .into_iter()
            .filter_map(|row| match (row.agent_instance_hierarchy, row.tag) {
                (Some(agent_instance_hierarchy), _) => Some(LaboratoryAttachment::Aih {
                    agent_instance_hierarchy,
                    attached_at: row.attached_at,
                    attached_by: row.attached_by,
                    machine: row.machine_id,
                    machine_state: row.machine_state,
                }),
                (None, Some(tag)) => Some(LaboratoryAttachment::Tag {
                    tag,
                    attached_at: row.attached_at,
                    attached_by: row.attached_by,
                    machine: row.machine_id,
                    machine_state: row.machine_state,
                }),
                // Both NULL is unrepresentable (table CHECK); skip
                // defensively rather than invent a target.
                (None, None) => None,
            })
            .collect();

        let (image, mounts, env, cwd, created_at, machine, machine_state) = match identity
        {
            Some((machine, state, identify)) => (
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
                identify.created_at,
                Some(machine),
                Some(state),
            ),
            None => (None, Vec::new(), Vec::new(), None, None, None, None),
        };
        Some(LaboratoryRecord {
            id: id.to_string(),
            image,
            mounts,
            env,
            cwd,
            created_at,
            machine,
            machine_state,
            connected,
            attachments,
        })
    }
}

/// [`Identify`] + its serving host (machine + state) → one list item.
fn status_from_identify(
    lab: Identify,
    machine: MachineIdentity,
    machine_state: String,
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
        created_at: lab.created_at,
        machine: Some(machine),
        machine_state: Some(machine_state),
        connected: true,
    }
}

/// The list pump's diff key: the FULL laboratory identity — ids are
/// only unique per (machine, state), so same-id items from different
/// hosts must not collide in the diff map.
fn status_key(status: &LaboratoryStatus) -> String {
    format!(
        "{}\n{}\n{}",
        status.machine.as_ref().map(|m| m.id.as_str()).unwrap_or(""),
        status.machine_state.as_deref().unwrap_or(""),
        status.id
    )
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

    let mut last: BTreeMap<String, LaboratoryStatus> = hub
        .list()
        .await
        .into_iter()
        .map(|lab| (status_key(&lab), lab))
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
                    Ok(LabsChange::Registry)
                    | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
                let next: BTreeMap<String, LaboratoryStatus> = hub
                    .list()
                    .await
                    .into_iter()
                    .map(|lab| (status_key(&lab), lab))
                    .collect();
                let mut frames: Vec<LaboratoryEvent> = Vec::new();
                for (key, lab) in &next {
                    if last.get(key) != Some(lab) {
                        frames.push(LaboratoryEvent::Upserted {
                            laboratory: lab.clone(),
                        });
                    }
                }
                for (key, old) in &last {
                    if !next.contains_key(key) {
                        // The pair rides the removal — a bare id is
                        // ambiguous when another host serves the same
                        // id.
                        frames.push(LaboratoryEvent::Removed {
                            id: old.id.clone(),
                            machine: old.machine.as_ref().map(|m| m.id.clone()),
                            machine_state: old.machine_state.clone(),
                        });
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

/// The `/laboratories/{*id}` route's optional query: pins the exact
/// laboratory (`?machine=…&machine_state=…`) — ids are only unique
/// per (machine, state). Anything short of the full pair is treated
/// as absent (legacy first-match-by-id).
#[derive(serde::Deserialize)]
pub(crate) struct RecordQuery {
    #[serde(default)]
    machine: Option<String>,
    #[serde(default)]
    machine_state: Option<String>,
}

/// `/laboratories/{*id}`: upgrade, consume the auth preamble, send
/// the record, then re-send it (full-value) on every relevant change.
pub(crate) async fn laboratory_instance_handler(
    axum::extract::State(state): axum::extract::State<
        crate::websockets::daemon_stream::DaemonWsState,
    >,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<RecordQuery>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        if !crate::websockets::daemon_auth::authenticate(&mut socket, state.secret.as_ref())
            .await
        {
            return;
        }
        let host = match (query.machine, query.machine_state) {
            (Some(machine), Some(machine_state)) => Some((machine, machine_state)),
            _ => None,
        };
        laboratory_instance_pump(socket, state.labs_hub, id, host).await;
    })
}

/// Send the current record, then rebuild on EVERY change (registry and
/// attachments both can alter it) and send only when the record
/// actually differs. A DB outage skips the frame; the next change
/// retries.
async fn laboratory_instance_pump(
    mut socket: axum::extract::ws::WebSocket,
    hub: LaboratoriesHub,
    id: String,
    host: Option<(String, String)>,
) {
    use axum::extract::ws::Message;
    let host = host
        .as_ref()
        .map(|(machine, machine_state)| (machine.as_str(), machine_state.as_str()));
    let mut rx = hub.subscribe();

    let mut last: Option<LaboratoryRecord> = hub.build_record(&id, host).await;
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
                let Some(record) = hub.build_record(&id, host).await else {
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
