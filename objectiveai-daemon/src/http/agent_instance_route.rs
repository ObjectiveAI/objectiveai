//! The resident daemon's live-conversation hub — the
//! `/agents/instances/{*aih}` endpoint.
//!
//! - **Producer side** — a fixed-name local socket
//!   (`<state>/socks/conversation.sock` on Unix; a namespaced pipe on
//!   Windows), SEPARATE from `daemon.sock` / `agents.sock`. Every log
//!   writer tees its admitted rows here as JSONL
//!   [`TeeFrame`](crate::db::logs::TeeFrame)s — keyed FULL-VALUE
//!   upserts, shipped before their SQL runs. One connection carries
//!   MANY AIHs (a function execution streams every nested agent's
//!   rows), so routing is per-frame by the row's own AIH, never
//!   per-connection.
//! - **Consumer side** — the [`axum`] `/agents/instances/{*aih}`
//!   SSE route (wildcard: AIHs contain `/`).
//!   On connect a client gets the agent's conversation replayed from
//!   the DB (content inlined, `objectiveai.messages."index"` order,
//!   paged), then [`AgentInstanceEvent::Live`], then live frames.
//!   Duplicates across the seam converge client-side by row identity.
//! - **Lag policy** — a slow client is DISCONNECTED (never
//!   drop-and-continue like `/listen`): dropped rows are only
//!   recoverable by reconnecting for a fresh snapshot, so the close
//!   IS the resync signal.

use std::sync::Arc;

use objectiveai_sdk::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};
use objectiveai_sdk::cli::agents_instances_listener::{
    AgentInstanceEvent, ClientNotificationPart, PartContent,
};
use sqlx::Row as _;
use tokio::sync::broadcast;

/// Snapshot page size — rows per DB round-trip while replaying a
/// conversation on connect. Bounds peak memory for huge histories.
const SNAPSHOT_PAGE: i64 = 5000;


/// The conversation fan-out: one global broadcast of
/// `(aih, pre-serialized AgentInstanceEvent JSON)` tuples; each
/// `/agents/instances/{*aih}` subscriber filters by exact AIH.
#[derive(Clone)]
pub(crate) struct ConversationHub {
    events: broadcast::Sender<(Arc<str>, Arc<str>)>,
    /// Resident context — DB pool resolved lazily (`db_client`), used
    /// to resolve message-queue notification content (the writer only
    /// knows the content id) and for connect-time snapshots.
    ctx: crate::context::Context,
}

impl ConversationHub {
    pub(crate) fn new(
        events: broadcast::Sender<(Arc<str>, Arc<str>)>,
        ctx: crate::context::Context,
    ) -> Self {
        Self { events, ctx }
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<(Arc<str>, Arc<str>)> {
        self.events.subscribe()
    }

    /// Serialize + fan one conversation event out, routed by the
    /// event's own AIH. A send error means no subscribers — drop the
    /// frame. `Live` / `Agent` carry no AIH and are never fanned out
    /// here (they are per-connection concerns).
    pub(crate) fn broadcast_event(&self, event: AgentInstanceEvent) {
        let Some(aih) = event_aih(&event) else {
            return;
        };
        let aih: Arc<str> = Arc::from(aih);
        if let Ok(json) = serde_json::to_string(&event) {
            let _ = self.events.send((aih, Arc::from(json.as_str())));
        }
    }

    /// Resolve an unresolved message-queue frame — content from the
    /// per-kind table, block metadata from the parent queue row — into
    /// a full [`ConversationRow`]. Best-effort: DB unavailable or the
    /// row missing drops the frame (notifications are low-frequency
    /// and land in reconnect snapshots regardless).
    pub(crate) async fn resolve_message_queue_content(
        &self,
        agent_instance_hierarchy: String,
        response_id: String,
        message_queue_content_id: i64,
        delivered_at: String,
    ) -> Option<AgentInstanceEvent> {
        let pool = self.ctx.db_client().await.ok()?;
        let row = sqlx::query(
            "SELECT mqc.kind::text AS kind, \
                    mq.id AS mq_id, \
                    mq.sender_agent_instance_hierarchy AS sender, \
                    mq.enqueued_at, \
                    mq.key AS mq_key, \
                    t.text AS text, \
                    i.url AS image_url, i.detail AS image_detail, \
                    a.data AS audio_data, a.format AS audio_format, \
                    v.url AS video_url, \
                    f.file_data, f.file_id, f.filename, f.file_url \
             FROM objectiveai.message_queue_contents mqc \
             JOIN objectiveai.message_queue mq ON mqc.message_queue_id = mq.id \
             LEFT JOIN objectiveai.message_queue_texts t  ON t.id = mqc.id \
             LEFT JOIN objectiveai.message_queue_images i ON i.id = mqc.id \
             LEFT JOIN objectiveai.message_queue_audios a ON a.id = mqc.id \
             LEFT JOIN objectiveai.message_queue_videos v ON v.id = mqc.id \
             LEFT JOIN objectiveai.message_queue_files f  ON f.id = mqc.id \
             WHERE mqc.id = $1",
        )
        .bind(message_queue_content_id)
        .fetch_optional(&**pool)
        .await
        .ok()??;

        let kind: String = row.try_get("kind").ok()?;
        let content = match kind.as_str() {
            "text" => PartContent::Text {
                text: row.try_get("text").ok()?,
            },
            "image" => {
                let url: String = row.try_get("image_url").ok()?;
                let detail_str: Option<String> = row.try_get("image_detail").ok()?;
                let detail = match detail_str {
                    Some(s) => serde_json::from_value(serde_json::Value::String(s)).ok()?,
                    None => None,
                };
                PartContent::Image(ImageUrl { url, detail })
            }
            "audio" => PartContent::Audio(InputAudio {
                data: row.try_get("audio_data").ok()?,
                format: row.try_get("audio_format").ok()?,
            }),
            "video" => PartContent::Video(VideoUrl {
                url: row.try_get("video_url").ok()?,
            }),
            "file" => PartContent::File(File {
                file_data: row.try_get("file_data").ok()?,
                file_id: row.try_get("file_id").ok()?,
                filename: row.try_get("filename").ok()?,
                file_url: row.try_get("file_url").ok()?,
            }),
            _ => return None,
        };
        let enqueued_at: Option<i64> = row.try_get("enqueued_at").ok()?;
        let sender: Option<String> = row.try_get("sender").ok()?;
        Some(AgentInstanceEvent::ClientNotificationPart {
            agent_instance_hierarchy,
            response_id,
            sender_agent_instance_hierarchy: sender.unwrap_or_default(),
            message_queue_id: row.try_get("mq_id").ok()?,
            queued_at: crate::db::time::unix_to_rfc3339(enqueued_at.unwrap_or_default()),
            key: row.try_get("mq_key").ok()?,
            row_index: message_queue_content_id,
            part: ClientNotificationPart {
                delivered_at,
                content,
            },
        })
    }
}

/// The conversation event's own AIH — every conversation-carrying
/// variant has one; `Live` / `Agent` do not (per-connection concerns,
/// never hub-routed).
fn event_aih(event: &AgentInstanceEvent) -> Option<&str> {
    use AgentInstanceEvent as E;
    match event {
        E::RequestMessageUserPart { agent_instance_hierarchy, .. }
        | E::RequestMessageAssistantPart { agent_instance_hierarchy, .. }
        | E::RequestMessageToolPart { agent_instance_hierarchy, .. }
        | E::VectorRequestChoicePart { agent_instance_hierarchy, .. }
        | E::VectorResponseVote { agent_instance_hierarchy, .. }
        | E::ClientNotificationPart { agent_instance_hierarchy, .. }
        | E::AssistantResponsePart { agent_instance_hierarchy, .. }
        | E::ToolResponsePart { agent_instance_hierarchy, .. }
        | E::Error { agent_instance_hierarchy, .. } => Some(agent_instance_hierarchy),
        E::Live | E::Agent { .. } => None,
    }
}


/// `/agents/instances/{*aih}`: header-auth, then an SSE stream that
/// replays the DB snapshot, marks live, and relays this AIH's frames.
pub(crate) async fn instance_handler(
    axum::extract::State(state): axum::extract::State<
        crate::http::daemon_stream::DaemonHttpState,
    >,
    axum::extract::Path(aih): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !crate::http::daemon_auth::authenticate_header(&headers, state.secret.as_ref()) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    axum::response::sse::Sse::new(instance_stream(
        state.conversations,
        state.active,
        aih,
    ))
    .keep_alive(axum::response::sse::KeepAlive::default())
    .into_response()
}

/// The snapshot→live seam, in order: subscribe FIRST on BOTH concerns
/// (no gap — duplicates converge client-side: conversation rows by
/// identity, the agent record by full-value replace), ship the agent's
/// current status record, replay the DB conversation snapshot page by
/// page (one SSE frame per row), send `Live`, then relay both streams.
/// A DB-less daemon skips the snapshot and goes straight to live.
/// Conversation `Lagged` ENDS the stream (the client resyncs on
/// reconnect); agent-status `Lagged` self-heals (full state is one
/// cheap query — and a fleet-wide burst of OTHER agents' events must
/// not kill this connection). A dropped stream (client gone) drops
/// both subscriptions.
fn instance_stream(
    hub: ConversationHub,
    active: crate::http::agents_routes::ActiveAgents,
    aih: String,
) -> impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>> {
    use axum::response::sse::Event;
    use objectiveai_sdk::cli::agents_instances_listener::AgentRecord;

    use crate::http::agents_routes::StatusChange;
    async_stream::stream! {
        let mut rx = hub.subscribe();
        let mut agents_rx = active.subscribe();

        // The agent's current status record, first — one small frame,
        // instant active/tags state, independent of the conversation.
        // (`get_exact` zero-fills unknown AIHs, so a record exists whenever
        // the DB is up; DB-less: skipped, the first live event covers it.)
        let mut last_record: Option<AgentRecord> = active.build_record_for(&aih).await;
        if let Some(agent) = &last_record {
            let event = AgentInstanceEvent::Agent {
                agent: agent.clone(),
            };
            if let Ok(frame) = serde_json::to_string(&event) {
                yield Ok(Event::default().data(frame));
            }
        }

        if let Ok(pool) = hub.ctx.db_client().await {
            let mut after_id: Option<i64> = None;
            loop {
                let page = crate::db::logs::read_conversation_page(
                    pool,
                    &aih,
                    after_id,
                    SNAPSHOT_PAGE,
                )
                .await;
                let (rows, next) = match page {
                    Ok(page) => page,
                    // Partial snapshot on a DB error: proceed live — the
                    // client re-snapshots on its next reconnect anyway.
                    Err(_) => break,
                };
                for event in rows {
                    let Ok(frame) = serde_json::to_string(&event) else {
                        continue;
                    };
                    yield Ok(Event::default().data(frame));
                }
                match next {
                    Some(next) => after_id = Some(next),
                    None => break,
                }
            }
        }

        if let Ok(live) = serde_json::to_string(&AgentInstanceEvent::Live) {
            yield Ok(Event::default().data(live));
        }

        loop {
            tokio::select! {
                received = rx.recv() => match received {
                    Ok((frame_aih, frame)) => {
                        if *frame_aih != *aih {
                            continue;
                        }
                        yield Ok(Event::default().data(frame.to_string()));
                    }
                    // Lagging client: rows were dropped and full-value
                    // upserts cannot recover a dropped FINAL state —
                    // end the stream so the client resyncs with a fresh
                    // snapshot. (Unlike /listen's drop-and-continue.)
                    Err(broadcast::error::RecvError::Lagged(_)) => break,
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                received = agents_rx.recv() => match received {
                    Ok(change) => {
                        // The internal status broadcast carries every
                        // agent's lifecycle/tag changes. Keep only this
                        // agent's, and ship a full-value `Agent` record.
                        let agent = match change {
                            StatusChange::Activated { agent_instance_hierarchy }
                            | StatusChange::TagsChanged { agent_instance_hierarchy }
                            | StatusChange::AttachmentsChanged { agent_instance_hierarchy }
                            | StatusChange::ActiveLaboratoriesChanged { agent_instance_hierarchy }
                                if agent_instance_hierarchy == aih =>
                            {
                                // Rebuild from DB truth + the live active
                                // flag (changes are low-frequency).
                                match active.build_record_for(&aih).await {
                                    Some(record) => Some(record),
                                    None => continue,
                                }
                            }
                            StatusChange::Deactivated {
                                agent_instance_hierarchy,
                                last_active_at,
                            } if agent_instance_hierarchy == aih => {
                                // Patch the last-known record with the
                                // release-moment timestamp (fall back to a
                                // fresh build for a connection that never
                                // got one — e.g. a DB-less connect).
                                let mut record = match last_record.take() {
                                    Some(record) => record,
                                    None => match active.build_record_for(&aih).await {
                                        Some(record) => record,
                                        None => continue,
                                    },
                                };
                                record.active = false;
                                record.last_active_at = last_active_at;
                                Some(record)
                            }
                            // Other agents' changes.
                            _ => None,
                        };
                        let Some(agent) = agent else { continue };
                        last_record = Some(agent.clone());
                        let event = AgentInstanceEvent::Agent { agent };
                        let Ok(frame) = serde_json::to_string(&event) else {
                            continue;
                        };
                        yield Ok(Event::default().data(frame));
                    }
                    // Agent-status lag self-heals: the full state is one
                    // cheap query, and a fleet-wide burst of OTHER agents'
                    // events must not kill this connection.
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if let Some(agent) = active.build_record_for(&aih).await {
                            last_record = Some(agent.clone());
                            let event = AgentInstanceEvent::Agent { agent };
                            if let Ok(frame) = serde_json::to_string(&event) {
                                yield Ok(Event::default().data(frame));
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                },
            }
        }
    }
}
