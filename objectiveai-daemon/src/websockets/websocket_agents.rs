//! The resident daemon's live agent-status hub — the `/agents/instances/list` endpoint.
//!
//! Robust active/inactive tracking, driven by the per-agent IN-PROCESS
//! lock ([`crate::command::agents::locks`]) rather than by stream
//! lifecycles:
//!
//! - **Producer side** — every place the daemon acquires an
//!   `agent_instance_hierarchy` (AIH) instance lock (via
//!   [`crate::websockets::agent_registry`]) calls [`ActiveAgents::activate`]
//!   directly: "AIH X is now active."
//! - **Watcher** — on each activation the daemon spawns
//!   [`ActiveAgents::watch`], which awaits the AIH lock's release
//!   ([`crate::command::agents::locks::wait_released`]). A guard drops when
//!   the agent's task ends (or the whole daemon dies), so a spawn killed
//!   mid-stream flips to inactive exactly — no leak, no reliance on a clean
//!   stream end.
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
use std::path::PathBuf;
use std::sync::Arc;

use objectiveai_sdk::cli::command::agents::instances::list::ResponseItem;
use objectiveai_sdk::cli::websocket_agents_instances_list_listener::{AgentEvent, AgentStatus};
use objectiveai_sdk::cli::websocket_agents_instances_listener::AgentRecord;
use tokio::sync::{Mutex, broadcast};


/// CLI-internal agent-status change, broadcast by [`ActiveAgents`] to
/// BOTH websocket routes, which map it to their own wire vocabularies:
/// `/agents/instances/list` re-ships `Activated`/`Deactivated` as its
/// flat status events (and ignores `TagsChanged`);
/// `/agents/instances/{*aih}` rebuilds and re-ships this one agent's
/// full record on any matching change.
#[derive(Debug, Clone)]
pub(crate) enum StatusChange {
    /// The AIH acquired its instance lock.
    Activated { agent_instance_hierarchy: String },
    /// The AIH released its instance lock (normal end or holder
    /// death). Carries the release-moment timestamp so the per-agent
    /// route can patch its record exactly.
    Deactivated {
        agent_instance_hierarchy: String,
        last_active_at: Option<String>,
    },
    /// The AIH's bound tags changed (applied / moved / removed).
    TagsChanged { agent_instance_hierarchy: String },
    /// The AIH's ATTACHED laboratory set changed (a lab was attached
    /// or detached, on the AIH itself or on one of its bound tags).
    AttachmentsChanged { agent_instance_hierarchy: String },
    /// The AIH's ACTIVE laboratory set changed (a spawn pass recorded
    /// the set it sent with its most recent request).
    ActiveLaboratoriesChanged { agent_instance_hierarchy: String },
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
    /// Typed [`StatusChange`]s, fanned to both websocket routes.
    events: broadcast::Sender<StatusChange>,
    state_dir: PathBuf,
    /// Resident context — the DB pool is resolved lazily (`db_client`), as
    /// the daemon boots before a DB necessarily exists.
    ctx: crate::context::Context,
}

impl ActiveAgents {
    pub(crate) fn new(
        state_dir: PathBuf,
        events: broadcast::Sender<StatusChange>,
        ctx: crate::context::Context,
    ) -> Self {
        Self {
            active: Arc::new(Mutex::new(HashSet::new())),
            events,
            state_dir,
            ctx,
        }
    }

    /// A fresh subscription to the status-change stream.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<StatusChange> {
        self.events.subscribe()
    }

    /// Fan one change out. A send error means no subscribers — drop it.
    fn emit(&self, change: StatusChange) {
        let _ = self.events.send(change);
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
        self.emit(StatusChange::Activated {
            agent_instance_hierarchy: aih.clone(),
        });
        let this = self.clone();
        tokio::spawn(async move { this.watch(aih).await });
    }

    /// Watch `aih`'s instance lock until it is released (or its holder
    /// dies), then flip it inactive. Re-probes held-state under the map
    /// lock so a reacquire during the wake gap keeps the AIH active with no
    /// spurious delta.
    async fn watch(self, aih: String) {
        let (dir, key) =
            crate::command::agents::locks::agent_instance_lock(&self.state_dir, &aih);
        loop {
            // Wakes when the AIH's in-process mutex is released.
            crate::command::agents::locks::wait_released(self.ctx.agent_locks(), &dir, &key).await;
            let mut active = self.active.lock().await;
            // A new holder may have acquired during the wake gap (fast
            // reacquire). Under the lock so `activate` cannot interleave
            // and lose the transition.
            if crate::command::agents::locks::try_held(self.ctx.agent_locks(), &dir, &key) {
                drop(active);
                continue;
            }
            active.remove(&aih);
            drop(active);
            let last =
                crate::db::time::unix_to_rfc3339(chrono::Utc::now().timestamp());
            self.emit(StatusChange::Deactivated {
                agent_instance_hierarchy: aih,
                last_active_at: Some(last),
            });
            break;
        }
    }

    /// The connect-time snapshot: every known AIH (from the DB), each
    /// with its `active` flag from the registry, plus any active AIH
    /// not yet in the DB (brand-new). Nothing but the AIH + flag —
    /// this endpoint's whole payload.
    async fn snapshot(&self) -> Vec<AgentStatus> {
        let items = match self.ctx.db_client().await {
            Ok(pool) => crate::db::instances::list_all(pool).await.unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let active = self.active.lock().await;
        let mut out = Vec::with_capacity(items.len());
        let mut seen: HashSet<&str> = HashSet::new();
        for item in &items {
            seen.insert(item.agent_instance_hierarchy.as_str());
            out.push(AgentStatus {
                agent_instance_hierarchy: item.agent_instance_hierarchy.clone(),
                active: active.contains(&item.agent_instance_hierarchy),
            });
        }
        for aih in active.iter() {
            if !seen.contains(aih.as_str()) {
                out.push(AgentStatus {
                    agent_instance_hierarchy: aih.clone(),
                    active: true,
                });
            }
        }
        out
    }

    /// Build the current record for `aih` — DB truth
    /// ([`crate::db::instances::get_exact`]) with the `active` flag from
    /// the registry (a live agent's `last_active_at` is suppressed).
    /// `None` if the DB is unavailable. Also used by the
    /// `/agents/instances/{*aih}` route for its per-agent status frames.
    pub(crate) async fn build_record_for(&self, aih: &str) -> Option<AgentRecord> {
        let active = self.active.lock().await.contains(aih);
        let pool = self.ctx.db_client().await.ok()?;
        let item = crate::db::instances::get_exact(pool, aih).await.ok()?;
        // ATTACHED laboratories — the effective union (AIH ∪ bound tags).
        let attached =
            crate::db::laboratory_attachments::effective_for_aih(pool, aih, &item.tags)
                .await
                .ok()?;
        // ACTIVE laboratories — what the most recent spawn pass sent.
        // A fully separate concern from the attachments above.
        let active_laboratories =
            crate::db::agent_active_laboratories::list(pool, aih).await.ok()?;
        Some(record_from_item(&item, active, attached, active_laboratories))
    }

    /// Subscribe to the `tags_changed` NOTIFY channel and emit a
    /// [`StatusChange::TagsChanged`] for each AIH whose bound tags
    /// changed (tag applied / moved / removed — a trigger on
    /// `objectiveai.tags` fires the AIH as payload). Consumed by the
    /// per-agent route only (the list endpoint carries no tags). Runs
    /// for the daemon's life; on a listener error it reconnects after
    /// a short pause. This is the persisted-state counterpart to the
    /// lock-driven active/inactive tracking: tags live in the DB, so
    /// the DB is the authoritative change signal.
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
                self.emit(StatusChange::TagsChanged {
                    agent_instance_hierarchy: notification.payload().to_string(),
                });
            }
            // Listener errored/closed — pause, then reconnect.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Subscribe to the `laboratory_attachments_changed` NOTIFY channel
    /// and emit a [`StatusChange::AttachmentsChanged`] for each affected
    /// AIH. The trigger payload preserves the row's target column
    /// (`aih:<value>` / `tag:<value>`); a tag payload resolves to its
    /// BOUND AIH here (GROUPED/absent tags map to no live record and are
    /// dropped, matching `effective_for_aih`'s read path). Same
    /// reconnect-loop shape as [`Self::watch_tag_changes`]; tracks the
    /// ATTACHED set only — the ACTIVE set is a separate watcher.
    pub(crate) async fn watch_attachment_changes(self) {
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
            while let Ok(notification) = listener.recv().await {
                let payload = notification.payload();
                if let Some(aih) = payload.strip_prefix("aih:") {
                    self.emit(StatusChange::AttachmentsChanged {
                        agent_instance_hierarchy: aih.to_string(),
                    });
                } else if let Some(tag) = payload.strip_prefix("tag:") {
                    let Ok(pool) = self.ctx.db_client().await else {
                        continue;
                    };
                    if let Ok(crate::db::tags::LookupState::Bound {
                        agent_instance_hierarchy,
                    }) = crate::db::tags::lookup(pool, tag).await
                    {
                        self.emit(StatusChange::AttachmentsChanged {
                            agent_instance_hierarchy,
                        });
                    }
                }
            }
            // Listener errored/closed — pause, then reconnect.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Subscribe to the `agent_active_laboratories_changed` NOTIFY
    /// channel and emit a [`StatusChange::ActiveLaboratoriesChanged`]
    /// per notification (payload = the AIH; fired once per spawn-pass
    /// replace). Same reconnect-loop shape as
    /// [`Self::watch_tag_changes`]; tracks the ACTIVE set only — the
    /// ATTACHED set is a separate watcher.
    pub(crate) async fn watch_active_laboratory_changes(self) {
        use std::time::Duration;
        loop {
            let reconnect = async {
                let pool = self.ctx.db_client().await.ok()?;
                let mut listener =
                    sqlx::postgres::PgListener::connect_with(&**pool).await.ok()?;
                listener.listen("agent_active_laboratories_changed").await.ok()?;
                Some(listener)
            }
            .await;
            let Some(mut listener) = reconnect else {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            };
            while let Ok(notification) = listener.recv().await {
                self.emit(StatusChange::ActiveLaboratoriesChanged {
                    agent_instance_hierarchy: notification.payload().to_string(),
                });
            }
            // Listener errored/closed — pause, then reconnect.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Best-effort startup reconcile: seed the registry with agents whose
    /// in-process lock is already held when the daemon starts (or before
    /// any client connects). Probes `try_held` per candidate AIH from
    /// `list_all`. Since agents are in-process tasks that die with the
    /// daemon, a freshly-booted daemon holds no agent mutex and this finds
    /// nothing — it stays for the mid-life "before first client" case and
    /// as a harmless invariant. Errors are ignored.
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
            if crate::command::agents::locks::try_held(self.ctx.agent_locks(), &dir, &key) {
                self.activate(item.agent_instance_hierarchy).await;
            }
        }
    }
}

/// Map an `agents instances list` item to an [`AgentRecord`]. `created_at`
/// becomes `spawned_at`; a live agent's `last_active_at` is suppressed
/// (implicitly "now").
fn record_from_item(
    item: &ResponseItem,
    active: bool,
    attached: Vec<crate::db::laboratory_attachments::AttachmentRecord>,
    active_laboratories: Vec<String>,
) -> AgentRecord {
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
        attached_laboratories: attached
            .into_iter()
            .map(|record| {
                objectiveai_sdk::cli::websocket_agents_instances_listener::AttachedLaboratory {
                    id: record.laboratory_id,
                    attached_at: crate::db::time::unix_to_rfc3339(record.attached_at),
                    attached_by: record.attached_by,
                }
            })
            .collect(),
        active_laboratories,
    }
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
                Ok(change) => {
                    // Map the internal change to this endpoint's flat
                    // wire vocabulary; tag changes don't ride the list.
                    let event = match change {
                        StatusChange::Activated { agent_instance_hierarchy } => {
                            AgentEvent::Activated { agent_instance_hierarchy }
                        }
                        StatusChange::Deactivated { agent_instance_hierarchy, .. } => {
                            AgentEvent::Deactivated { agent_instance_hierarchy }
                        }
                        StatusChange::TagsChanged { .. }
                        | StatusChange::AttachmentsChanged { .. }
                        | StatusChange::ActiveLaboratoriesChanged { .. } => continue,
                    };
                    let Ok(frame) = serde_json::to_string(&event) else {
                        continue;
                    };
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
