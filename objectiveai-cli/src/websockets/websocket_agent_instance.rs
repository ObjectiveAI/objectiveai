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
//! - **Consumer side** — the [`axum`] WebSocket
//!   `/agents/instances/{*aih}` route (wildcard: AIHs contain `/`).
//!   On connect a client gets the agent's conversation replayed from
//!   the DB (content inlined, `objectiveai.messages."index"` order,
//!   paged), then [`AgentInstanceEvent::Live`], then live frames.
//!   Duplicates across the seam converge client-side by row identity.
//! - **Lag policy** — a slow client is DISCONNECTED (never
//!   drop-and-continue like `/listen`): dropped rows are only
//!   recoverable by reconnecting for a fresh snapshot, so the close
//!   IS the resync signal.
//!
//! The inbound WS leg is read (and ignored beyond close detection) —
//! reserved for the planned client→daemon message requests over this
//! stream.

use std::path::Path;
use std::sync::Arc;

#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{ListenerOptions, Name};
use objectiveai_sdk::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};
use objectiveai_sdk::cli::websocket_agents_instances_listener::{
    AgentInstanceEvent, ConversationRow, RowContent, RowTableKind,
};
use sqlx::Row as _;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::broadcast;

use crate::db::logs::TeeFrame;
use crate::websockets::mcp_listener::socks_dir;

/// Snapshot page size — rows per DB round-trip while replaying a
/// conversation on connect. Bounds peak memory for huge histories.
const SNAPSHOT_PAGE: i64 = 5000;

/// The fixed local-socket name for the conversation hub — MUST match
/// the producer side in `crate::db::logs::tee::socket_name`. Mirrors
/// the `daemon.sock` / `agents.sock` scheme with the constant
/// `conversation`.
#[cfg(unix)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    socks_dir(state_dir)
        .join("conversation.sock")
        .to_fs_name::<GenericFilePath>()
}

#[cfg(windows)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    state_dir.file_name().hash(&mut hasher);
    let state = hasher.finish();
    format!("objectiveai-{state:016x}-conversation.sock").to_ns_name::<GenericNamespaced>()
}

/// Bind the fixed-name conversation producer socket. Bound
/// **synchronously** under the daemon init gate (like the daemon /
/// agents sockets) so a held daemon lock guarantees it is up.
pub fn bind_conversation_socket_listener(
    state_dir: &Path,
) -> std::io::Result<interprocess::local_socket::tokio::Listener> {
    let _ = std::fs::create_dir_all(socks_dir(state_dir));
    let name = socket_name(state_dir)?;
    ListenerOptions::new()
        .name(name)
        .try_overwrite(true)
        .create_tokio()
}

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

    /// Serialize + fan one row out. A send error means no subscribers
    /// — drop the frame.
    fn broadcast_row(&self, row: ConversationRow) {
        let aih: Arc<str> = Arc::from(row.agent_instance_hierarchy.as_str());
        let event = AgentInstanceEvent::Row { row };
        if let Ok(json) = serde_json::to_string(&event) {
            let _ = self.events.send((aih, Arc::from(json.as_str())));
        }
    }

    /// Resolve an unresolved message-queue frame — content from the
    /// per-kind table, block metadata from the parent queue row — into
    /// a full [`ConversationRow`]. Best-effort: DB unavailable or the
    /// row missing drops the frame (notifications are low-frequency
    /// and land in reconnect snapshots regardless).
    async fn resolve_message_queue_content(
        &self,
        agent_instance_hierarchy: String,
        response_id: String,
        message_queue_content_id: i64,
        delivered_at: String,
    ) -> Option<ConversationRow> {
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
        let (table, content) = match kind.as_str() {
            "text" => (
                RowTableKind::MessageQueueText,
                RowContent::Text {
                    text: row.try_get("text").ok()?,
                },
            ),
            "image" => {
                let url: String = row.try_get("image_url").ok()?;
                let detail_str: Option<String> = row.try_get("image_detail").ok()?;
                let detail = match detail_str {
                    Some(s) => serde_json::from_value(serde_json::Value::String(s)).ok()?,
                    None => None,
                };
                (
                    RowTableKind::MessageQueueImage,
                    RowContent::Image(ImageUrl { url, detail }),
                )
            }
            "audio" => (
                RowTableKind::MessageQueueAudio,
                RowContent::Audio(InputAudio {
                    data: row.try_get("audio_data").ok()?,
                    format: row.try_get("audio_format").ok()?,
                }),
            ),
            "video" => (
                RowTableKind::MessageQueueVideo,
                RowContent::Video(VideoUrl {
                    url: row.try_get("video_url").ok()?,
                }),
            ),
            "file" => (
                RowTableKind::MessageQueueFile,
                RowContent::File(File {
                    file_data: row.try_get("file_data").ok()?,
                    file_id: row.try_get("file_id").ok()?,
                    filename: row.try_get("filename").ok()?,
                    file_url: row.try_get("file_url").ok()?,
                }),
            ),
            _ => return None,
        };
        let enqueued_at: Option<i64> = row.try_get("enqueued_at").ok()?;
        Some(ConversationRow {
            agent_instance_hierarchy,
            response_id,
            table,
            row_index: message_queue_content_id,
            row_sub_index: None,
            delivered_at,
            tool_call_id: None,
            choice_key: None,
            sender_agent_instance_hierarchy: row.try_get("sender").ok()?,
            queued_at: crate::db::time::unix_to_rfc3339_opt(enqueued_at),
            message_queue_key: row.try_get("mq_key").ok()?,
            message_queue_id: row.try_get("mq_id").ok()?,
            content,
        })
    }
}

/// Spawn the accept loop on the pre-bound conversation socket: one
/// task per producer connection, each a long-lived JSONL line loop
/// (a writer streams frames for its whole spawn/execution lifetime).
pub fn serve_conversation_socket_listener(
    listener: interprocess::local_socket::tokio::Listener,
    hub: ConversationHub,
) {
    tokio::spawn(async move {
        loop {
            let conn = match listener.accept().await {
                Ok(conn) => conn,
                // Transient accept error — keep serving.
                Err(_) => continue,
            };
            let hub = hub.clone();
            tokio::spawn(async move {
                let (read_half, _write_half) = tokio::io::split(conn);
                let mut reader = BufReader::new(read_half);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF: writer closed.
                        Ok(_) => {}
                        Err(_) => break,
                    }
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    // Skip a malformed line rather than tearing down
                    // the stream (daemon_stream::handle_feed pattern).
                    let Ok(frame) = serde_json::from_str::<TeeFrame>(trimmed) else {
                        continue;
                    };
                    match frame {
                        TeeFrame::Row { row } => hub.broadcast_row(row),
                        TeeFrame::MessageQueueContent {
                            agent_instance_hierarchy,
                            response_id,
                            message_queue_content_id,
                            delivered_at,
                        } => {
                            if let Some(row) = hub
                                .resolve_message_queue_content(
                                    agent_instance_hierarchy,
                                    response_id,
                                    message_queue_content_id,
                                    delivered_at,
                                )
                                .await
                            {
                                hub.broadcast_row(row);
                            }
                        }
                    }
                }
            });
        }
    });
}

/// `/agents/instances/{*aih}`: upgrade, consume the auth preamble,
/// replay the DB snapshot, mark live, relay this AIH's frames.
pub(crate) async fn instance_handler(
    axum::extract::State(state): axum::extract::State<
        crate::websockets::daemon_stream::DaemonWsState,
    >,
    axum::extract::Path(aih): axum::extract::Path<String>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        if !crate::websockets::daemon_auth::authenticate(&mut socket, state.secret.as_ref())
            .await
        {
            return;
        }
        instance_pump(socket, state.conversations, state.active, aih).await;
    })
}

/// The snapshot→live seam, in order: subscribe FIRST on BOTH concerns
/// (no gap — duplicates converge client-side: conversation rows by
/// identity, the agent record by full-value replace), ship the agent's
/// current status record, replay the DB conversation snapshot page by
/// page (one WS text frame per row), send `Live`, then relay both
/// streams. A DB-less daemon skips the snapshot and goes straight to
/// live. Conversation `Lagged` DISCONNECTS the client; agent-status
/// `Lagged` self-heals (full state is one cheap query — and a
/// fleet-wide burst of OTHER agents' events must not kill this
/// connection).
async fn instance_pump(
    mut socket: axum::extract::ws::WebSocket,
    hub: ConversationHub,
    active: crate::websockets::websocket_agents::ActiveAgents,
    aih: String,
) {
    use axum::extract::ws::Message;
    use objectiveai_sdk::cli::websocket_agents_instances_list_listener::{AgentEvent, AgentRecord};
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
            if socket.send(Message::Text(frame.into())).await.is_err() {
                return;
            }
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
            for row in rows {
                let event = AgentInstanceEvent::Row { row };
                let Ok(frame) = serde_json::to_string(&event) else {
                    continue;
                };
                if socket.send(Message::Text(frame.into())).await.is_err() {
                    return;
                }
            }
            match next {
                Some(next) => after_id = Some(next),
                None => break,
            }
        }
    }

    let Ok(live) = serde_json::to_string(&AgentInstanceEvent::Live) else {
        return;
    };
    if socket.send(Message::Text(live.into())).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            received = rx.recv() => match received {
                Ok((frame_aih, frame)) => {
                    if *frame_aih != *aih {
                        continue;
                    }
                    if socket
                        .send(Message::Text(frame.to_string().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                // Lagging client: rows were dropped and full-value
                // upserts cannot recover a dropped FINAL state —
                // disconnect so the client resyncs with a fresh
                // snapshot. (Unlike /listen's drop-and-continue.)
                Err(broadcast::error::RecvError::Lagged(_)) => break,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            received = agents_rx.recv() => match received {
                Ok(frame) => {
                    // The list broadcast carries every agent's
                    // lifecycle/tag events, pre-serialized. Parse,
                    // keep only this agent's, and re-ship as a
                    // full-value `Agent` record.
                    let Ok(event) = serde_json::from_str::<AgentEvent>(&frame) else {
                        continue;
                    };
                    let agent = match event {
                        AgentEvent::Activated { agent } | AgentEvent::Updated { agent }
                            if agent.agent_instance_hierarchy == aih =>
                        {
                            Some(agent)
                        }
                        AgentEvent::Deactivated {
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
                        // Other agents' events, or shapes this route
                        // doesn't re-ship (Snapshot never broadcasts).
                        _ => None,
                    };
                    let Some(agent) = agent else { continue };
                    last_record = Some(agent.clone());
                    let event = AgentInstanceEvent::Agent { agent };
                    let Ok(frame) = serde_json::to_string(&event) else {
                        continue;
                    };
                    if socket.send(Message::Text(frame.into())).await.is_err() {
                        break;
                    }
                }
                // Agent-status lag self-heals: the full state is one
                // cheap query, and a fleet-wide burst of OTHER agents'
                // events must not kill this connection.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    if let Some(agent) = active.build_record_for(&aih).await {
                        last_record = Some(agent.clone());
                        let event = AgentInstanceEvent::Agent { agent };
                        if let Ok(frame) = serde_json::to_string(&event) {
                            if socket.send(Message::Text(frame.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            inbound = socket.recv() => match inbound {
                // Client closed or errored.
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                // Ignore other inbound messages — reserved for the
                // planned client message requests over this stream.
                Some(Ok(_)) => {}
            },
        }
    }
}
