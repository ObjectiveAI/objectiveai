//! Content-inlined conversation snapshot backing the daemon's
//! `/agents/instances/{*aih}` route.
//!
//! The same metadata query as [`super::read_all`] (shared
//! `SELECT_SHAPE` / `FROM_JOINS` / [`MsgRow`]), but instead of
//! emitting `{id, type}` parts for `agents logs read id` to resolve,
//! each row's ACTUAL content is batch-fetched from its per-kind table
//! and inlined into a typed SDK
//! [`AgentInstanceEvent`](objectiveai_sdk::cli::websocket_agents_instances_listener::AgentInstanceEvent)
//! — the same frame shape the live tee ships, so the WS handler
//! replays the snapshot and relays live frames through one type and
//! clients converge the seam by part identity.
//!
//! Batching: one metadata page (`"index"` ASC), then ONE query per
//! content family present in the page (`unnest` key joins /
//! `id = ANY` for the message-queue and error kinds) — never a
//! per-row round trip. Request-blob rows are skipped (hidden from
//! conversations, exactly as `read_all` hides them); a metadata row
//! whose content row is missing (torn write) skips that row.

use std::collections::HashMap;

use objectiveai_sdk::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};
use objectiveai_sdk::cli::websocket_agents_instances_listener::{
    AgentInstanceEvent, AssistantResponsePart, ClientNotificationPart, PartContent,
    RequestMessageUserPart, ToolResponsePart, VectorRequestChoicePart,
};
use sqlx::Row as _;

use super::super::time::unix_to_rfc3339;
use super::super::{Error, Pool};
use super::read_all::{FROM_JOINS, MsgRow, SELECT_SHAPE, row_into_msg};
use super::row::MessageTable;

/// Which media columns a content table carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ContentKind {
    Text,
    Image,
    Audio,
    Video,
    File,
}

/// Where one metadata row's content lives.
enum Source {
    /// `(response_id, "index", part_index)`-keyed content-part table.
    Part { table: &'static str, kind: ContentKind },
    /// `(response_id, "index")`-keyed single-text table
    /// (refusal / reasoning).
    IndexedText { table: &'static str },
    /// `(response_id, "index", tool_call_index)`-keyed tool-call table
    /// — arguments + call metadata.
    ToolCalls { table: &'static str },
    /// `id`-keyed per-kind message-queue content table
    /// (`row_index` = `message_queue_contents.id`).
    MessageQueue { table: &'static str, kind: ContentKind },
    /// `id`-keyed `objectiveai.errors` row (`row_index` = `errors.id`).
    ErrorRow,
    /// Inline on the metadata row (`MsgRow::vote`).
    Vote,
    /// Request-blob marker — hidden from conversations.
    Blob,
}

/// Content source per `objectiveai.message_table` kind. Table/column
/// names mirror [`super::read_id`]'s per-kind loaders exactly.
fn source(t: MessageTable) -> Source {
    use ContentKind as K;
    use MessageTable as T;
    match t {
        T::AgentCompletionRequest
        | T::VectorCompletionRequest
        | T::FunctionExecutionRequest => Source::Blob,
        T::MessageQueueText => Source::MessageQueue {
            table: "objectiveai.message_queue_texts",
            kind: K::Text,
        },
        T::MessageQueueImage => Source::MessageQueue {
            table: "objectiveai.message_queue_images",
            kind: K::Image,
        },
        T::MessageQueueAudio => Source::MessageQueue {
            table: "objectiveai.message_queue_audios",
            kind: K::Audio,
        },
        T::MessageQueueVideo => Source::MessageQueue {
            table: "objectiveai.message_queue_videos",
            kind: K::Video,
        },
        T::MessageQueueFile => Source::MessageQueue {
            table: "objectiveai.message_queue_files",
            kind: K::File,
        },
        T::AssistantResponseRefusal => Source::IndexedText {
            table: "objectiveai.assistant_response_refusal",
        },
        T::AssistantResponseReasoning => Source::IndexedText {
            table: "objectiveai.assistant_response_reasoning",
        },
        T::AssistantResponseToolCalls => Source::ToolCalls {
            table: "objectiveai.assistant_response_tool_calls",
        },
        T::AssistantResponseContentText => Source::Part {
            table: "objectiveai.assistant_response_content_text",
            kind: K::Text,
        },
        T::AssistantResponseContentImage => Source::Part {
            table: "objectiveai.assistant_response_content_image",
            kind: K::Image,
        },
        T::AssistantResponseContentAudio => Source::Part {
            table: "objectiveai.assistant_response_content_audio",
            kind: K::Audio,
        },
        T::AssistantResponseContentVideo => Source::Part {
            table: "objectiveai.assistant_response_content_video",
            kind: K::Video,
        },
        T::AssistantResponseContentFile => Source::Part {
            table: "objectiveai.assistant_response_content_file",
            kind: K::File,
        },
        T::ToolResponseContentText => Source::Part {
            table: "objectiveai.tool_response_content_text",
            kind: K::Text,
        },
        T::ToolResponseContentImage => Source::Part {
            table: "objectiveai.tool_response_content_image",
            kind: K::Image,
        },
        T::ToolResponseContentAudio => Source::Part {
            table: "objectiveai.tool_response_content_audio",
            kind: K::Audio,
        },
        T::ToolResponseContentVideo => Source::Part {
            table: "objectiveai.tool_response_content_video",
            kind: K::Video,
        },
        T::ToolResponseContentFile => Source::Part {
            table: "objectiveai.tool_response_content_file",
            kind: K::File,
        },
        T::RequestMessageUserContentText => Source::Part {
            table: "objectiveai.request_message_user_content_text",
            kind: K::Text,
        },
        T::RequestMessageUserContentImage => Source::Part {
            table: "objectiveai.request_message_user_content_image",
            kind: K::Image,
        },
        T::RequestMessageUserContentAudio => Source::Part {
            table: "objectiveai.request_message_user_content_audio",
            kind: K::Audio,
        },
        T::RequestMessageUserContentVideo => Source::Part {
            table: "objectiveai.request_message_user_content_video",
            kind: K::Video,
        },
        T::RequestMessageUserContentFile => Source::Part {
            table: "objectiveai.request_message_user_content_file",
            kind: K::File,
        },
        T::RequestMessageAssistantRefusal => Source::IndexedText {
            table: "objectiveai.request_message_assistant_refusal",
        },
        T::RequestMessageAssistantReasoning => Source::IndexedText {
            table: "objectiveai.request_message_assistant_reasoning",
        },
        T::RequestMessageAssistantToolCalls => Source::ToolCalls {
            table: "objectiveai.request_message_assistant_tool_calls",
        },
        T::RequestMessageAssistantContentText => Source::Part {
            table: "objectiveai.request_message_assistant_content_text",
            kind: K::Text,
        },
        T::RequestMessageAssistantContentImage => Source::Part {
            table: "objectiveai.request_message_assistant_content_image",
            kind: K::Image,
        },
        T::RequestMessageAssistantContentAudio => Source::Part {
            table: "objectiveai.request_message_assistant_content_audio",
            kind: K::Audio,
        },
        T::RequestMessageAssistantContentVideo => Source::Part {
            table: "objectiveai.request_message_assistant_content_video",
            kind: K::Video,
        },
        T::RequestMessageAssistantContentFile => Source::Part {
            table: "objectiveai.request_message_assistant_content_file",
            kind: K::File,
        },
        T::RequestMessageToolContentText => Source::Part {
            table: "objectiveai.request_message_tool_content_text",
            kind: K::Text,
        },
        T::RequestMessageToolContentImage => Source::Part {
            table: "objectiveai.request_message_tool_content_image",
            kind: K::Image,
        },
        T::RequestMessageToolContentAudio => Source::Part {
            table: "objectiveai.request_message_tool_content_audio",
            kind: K::Audio,
        },
        T::RequestMessageToolContentVideo => Source::Part {
            table: "objectiveai.request_message_tool_content_video",
            kind: K::Video,
        },
        T::RequestMessageToolContentFile => Source::Part {
            table: "objectiveai.request_message_tool_content_file",
            kind: K::File,
        },
        T::RequestVectorChoiceContentText => Source::Part {
            table: "objectiveai.request_vector_choice_content_text",
            kind: K::Text,
        },
        T::RequestVectorChoiceContentImage => Source::Part {
            table: "objectiveai.request_vector_choice_content_image",
            kind: K::Image,
        },
        T::RequestVectorChoiceContentAudio => Source::Part {
            table: "objectiveai.request_vector_choice_content_audio",
            kind: K::Audio,
        },
        T::RequestVectorChoiceContentVideo => Source::Part {
            table: "objectiveai.request_vector_choice_content_video",
            kind: K::Video,
        },
        T::RequestVectorChoiceContentFile => Source::Part {
            table: "objectiveai.request_vector_choice_content_file",
            kind: K::File,
        },
        T::ResponseVectorVote => Source::Vote,
        T::Error => Source::ErrorRow,
    }
}

/// One fetched content value, keyed by the metadata row's
/// `(table, response_id, row_index, row_sub_index)`.
enum Fetched {
    /// A media content part.
    Media(PartContent),
    /// A refusal / reasoning text.
    Text(String),
    /// A tool call's full state.
    ToolCall {
        tool_call_id: String,
        function_name: String,
        arguments: String,
    },
}

type ContentMap = HashMap<(MessageTable, String, i64, Option<i64>), Fetched>;

/// Build the per-kind media content from one fetched content row.
/// Column shapes are identical across every table of a kind — the
/// same contract [`super::read_id`] relies on.
fn media_content(kind: ContentKind, row: &sqlx::postgres::PgRow) -> Result<PartContent, Error> {
    Ok(match kind {
        ContentKind::Text => PartContent::Text {
            text: row.try_get("text")?,
        },
        ContentKind::Image => {
            let url: String = row.try_get("url")?;
            let detail_str: Option<String> = row.try_get("detail")?;
            let detail = match detail_str {
                Some(s) => serde_json::from_value(serde_json::Value::String(s))?,
                None => None,
            };
            PartContent::Image(ImageUrl { url, detail })
        }
        ContentKind::Audio => PartContent::Audio(InputAudio {
            data: row.try_get("data")?,
            format: row.try_get("format")?,
        }),
        ContentKind::Video => PartContent::Video(VideoUrl {
            url: row.try_get("url")?,
        }),
        ContentKind::File => PartContent::File(File {
            file_data: row.try_get("file_data")?,
            file_id: row.try_get("file_id")?,
            filename: row.try_get("filename")?,
            file_url: row.try_get("file_url")?,
        }),
    })
}

/// The columns to select for a media kind.
fn media_columns(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Text => "text",
        ContentKind::Image => "url, detail",
        ContentKind::Audio => "data, format",
        ContentKind::Video => "url",
        ContentKind::File => "file_data, file_id, filename, file_url",
    }
}

/// An assistant-part from its media content.
fn assistant_media(delivered_at: String, content: PartContent) -> AssistantResponsePart {
    match content {
        PartContent::Text { text } => AssistantResponsePart::Text { delivered_at, text },
        PartContent::Image(image) => AssistantResponsePart::Image {
            delivered_at,
            image,
        },
        PartContent::Audio(audio) => AssistantResponsePart::Audio {
            delivered_at,
            audio,
        },
        PartContent::Video(video) => AssistantResponsePart::Video {
            delivered_at,
            video,
        },
        PartContent::File(file) => AssistantResponsePart::File { delivered_at, file },
    }
}

/// One page of an agent's conversation as typed events, content
/// inlined, in `objectiveai.messages."index"` order. Returns the
/// events plus the `after_id` cursor for the next page (`None` when
/// this page was the last). The caller (the daemon WS handler) loops
/// pages, streaming each event as one frame — bounded memory for huge
/// histories.
pub async fn read_conversation_page(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    after_id: Option<i64>,
    limit: i64,
) -> Result<(Vec<AgentInstanceEvent>, Option<i64>), Error> {
    // Phase 1 — one metadata page via the shared read_all query.
    let sql = format!(
        "{SELECT_SHAPE} {FROM_JOINS} \
         WHERE m.agent_instance_hierarchy = $1 AND m.\"index\" > $2 \
         ORDER BY m.\"index\" ASC \
         LIMIT $3",
    );
    let rows = sqlx::query(&sql)
        .bind(agent_instance_hierarchy)
        .bind(after_id.unwrap_or(0))
        .bind(limit)
        .fetch_all(&**pool)
        .await?;
    let msgs = rows
        .iter()
        .map(row_into_msg)
        .collect::<Result<Vec<MsgRow>, Error>>()?;
    let next_after_id = if msgs.len() as i64 == limit {
        msgs.last().map(|m| m.id)
    } else {
        None
    };

    // Phase 2 — group the page's rows by content source and batch-fetch
    // each group in one query.
    let mut part_keys: HashMap<MessageTable, (Vec<String>, Vec<i64>, Vec<i64>)> = HashMap::new();
    let mut text_keys: HashMap<MessageTable, (Vec<String>, Vec<i64>)> = HashMap::new();
    let mut call_keys: HashMap<MessageTable, (Vec<String>, Vec<i64>, Vec<i64>)> = HashMap::new();
    let mut queue_ids: HashMap<MessageTable, Vec<i64>> = HashMap::new();
    let mut error_ids: Vec<i64> = Vec::new();
    for msg in &msgs {
        // Every content-fetch kind carries a response_id (only
        // `error` rows may lack one, and those fetch by id alone).
        let rid = msg.response_id.clone().unwrap_or_default();
        match source(msg.table_kind) {
            Source::Part { .. } => {
                let (Some(index), Some(part_index)) = (msg.row_index, msg.row_sub_index) else {
                    continue;
                };
                let entry = part_keys.entry(msg.table_kind).or_default();
                entry.0.push(rid);
                entry.1.push(index);
                entry.2.push(part_index);
            }
            Source::IndexedText { .. } => {
                let Some(index) = msg.row_index else { continue };
                let entry = text_keys.entry(msg.table_kind).or_default();
                entry.0.push(rid);
                entry.1.push(index);
            }
            Source::ToolCalls { .. } => {
                let (Some(index), Some(tool_call_index)) = (msg.row_index, msg.row_sub_index)
                else {
                    continue;
                };
                let entry = call_keys.entry(msg.table_kind).or_default();
                entry.0.push(rid);
                entry.1.push(index);
                entry.2.push(tool_call_index);
            }
            Source::MessageQueue { .. } => {
                let Some(id) = msg.row_index else { continue };
                queue_ids.entry(msg.table_kind).or_default().push(id);
            }
            Source::ErrorRow => {
                let Some(id) = msg.row_index else { continue };
                error_ids.push(id);
            }
            Source::Vote | Source::Blob => {}
        }
    }

    let mut content: ContentMap = HashMap::new();
    let mut errors_by_id: HashMap<i64, serde_json::Value> = HashMap::new();

    for (kind_table, (rids, indices, part_indices)) in &part_keys {
        let Source::Part { table, kind } = source(*kind_table) else {
            unreachable!("grouped as Part above");
        };
        let sql = format!(
            "SELECT t.response_id, t.\"index\" AS idx, t.part_index AS pidx, {} \
             FROM {table} t \
             JOIN unnest($1::text[], $2::bigint[], $3::bigint[]) AS k(rid, idx, pidx) \
               ON t.response_id = k.rid AND t.\"index\" = k.idx AND t.part_index = k.pidx",
            media_columns(kind),
        );
        for row in sqlx::query(&sql)
            .bind(rids)
            .bind(indices)
            .bind(part_indices)
            .fetch_all(&**pool)
            .await?
        {
            let rid: String = row.try_get("response_id")?;
            let idx: i64 = row.try_get("idx")?;
            let pidx: i64 = row.try_get("pidx")?;
            content.insert(
                (*kind_table, rid, idx, Some(pidx)),
                Fetched::Media(media_content(kind, &row)?),
            );
        }
    }

    for (kind_table, (rids, indices)) in &text_keys {
        let Source::IndexedText { table } = source(*kind_table) else {
            unreachable!("grouped as IndexedText above");
        };
        let sql = format!(
            "SELECT t.response_id, t.\"index\" AS idx, t.text \
             FROM {table} t \
             JOIN unnest($1::text[], $2::bigint[]) AS k(rid, idx) \
               ON t.response_id = k.rid AND t.\"index\" = k.idx",
        );
        for row in sqlx::query(&sql)
            .bind(rids)
            .bind(indices)
            .fetch_all(&**pool)
            .await?
        {
            let rid: String = row.try_get("response_id")?;
            let idx: i64 = row.try_get("idx")?;
            let text: String = row.try_get("text")?;
            content.insert((*kind_table, rid, idx, None), Fetched::Text(text));
        }
    }

    for (kind_table, (rids, indices, call_indices)) in &call_keys {
        let Source::ToolCalls { table } = source(*kind_table) else {
            unreachable!("grouped as ToolCalls above");
        };
        let sql = format!(
            "SELECT t.response_id, t.\"index\" AS idx, t.tool_call_index AS tci, \
                    t.tool_call_id, t.function_name, t.arguments \
             FROM {table} t \
             JOIN unnest($1::text[], $2::bigint[], $3::bigint[]) AS k(rid, idx, tci) \
               ON t.response_id = k.rid AND t.\"index\" = k.idx AND t.tool_call_index = k.tci",
        );
        for row in sqlx::query(&sql)
            .bind(rids)
            .bind(indices)
            .bind(call_indices)
            .fetch_all(&**pool)
            .await?
        {
            let rid: String = row.try_get("response_id")?;
            let idx: i64 = row.try_get("idx")?;
            let tci: i64 = row.try_get("tci")?;
            content.insert(
                (*kind_table, rid, idx, Some(tci)),
                Fetched::ToolCall {
                    tool_call_id: row.try_get("tool_call_id")?,
                    function_name: row.try_get("function_name")?,
                    arguments: row.try_get("arguments")?,
                },
            );
        }
    }

    for (kind_table, ids) in &queue_ids {
        let Source::MessageQueue { table, kind } = source(*kind_table) else {
            unreachable!("grouped as MessageQueue above");
        };
        let sql = format!(
            "SELECT t.id, {} FROM {table} t WHERE t.id = ANY($1::bigint[])",
            media_columns(kind),
        );
        for row in sqlx::query(&sql).bind(ids).fetch_all(&**pool).await? {
            let id: i64 = row.try_get("id")?;
            // Notification rows key by the content id in `row_index`;
            // response_id is still part of the identity tuple.
            for msg in msgs
                .iter()
                .filter(|m| m.table_kind == *kind_table && m.row_index == Some(id))
            {
                content.insert(
                    (
                        *kind_table,
                        msg.response_id.clone().unwrap_or_default(),
                        id,
                        None,
                    ),
                    Fetched::Media(media_content(kind, &row)?),
                );
            }
        }
    }

    if !error_ids.is_empty() {
        for row in
            sqlx::query("SELECT id, error FROM objectiveai.errors WHERE id = ANY($1::bigint[])")
                .bind(&error_ids)
                .fetch_all(&**pool)
                .await?
        {
            let id: i64 = row.try_get("id")?;
            let error: serde_json::Value = row.try_get("error")?;
            errors_by_id.insert(id, error);
        }
    }

    // Phase 3 — emit typed events in index order, blobs skipped,
    // content inlined.
    let mut out = Vec::with_capacity(msgs.len());
    for msg in &msgs {
        let delivered_at = unix_to_rfc3339(msg.timestamp_delivered);
        let aih = msg.agent_instance_hierarchy.clone();

        // `error` first — the one kind whose response_id may be NULL.
        if msg.table_kind == MessageTable::Error {
            let Some(error) = msg.row_index.and_then(|id| errors_by_id.get(&id)) else {
                continue; // torn write
            };
            out.push(AgentInstanceEvent::Error {
                agent_instance_hierarchy: aih,
                response_id: msg.response_id.clone(),
                error: error.clone(),
                delivered_at,
            });
            continue;
        }

        // Every other kind requires its response_id.
        let Some(response_id) = msg.response_id.clone() else {
            continue;
        };

        match source(msg.table_kind) {
            Source::Blob | Source::ErrorRow => continue,
            Source::Vote => {
                let Some(vote) = msg
                    .vote
                    .clone()
                    .and_then(|v| serde_json::from_value(v).ok())
                else {
                    continue;
                };
                out.push(AgentInstanceEvent::VectorResponseVote {
                    agent_instance_hierarchy: aih,
                    response_id,
                    vote,
                });
            }
            _ => {
                let row_index = msg.row_index.unwrap_or(0);
                let key = (
                    msg.table_kind,
                    response_id.clone(),
                    row_index,
                    msg.row_sub_index,
                );
                let Some(fetched) = content.get(&key) else {
                    // Torn write (metadata row without its content
                    // row) — skip, exactly like a missing read_id.
                    continue;
                };
                let Some(event) = build_event(
                    msg,
                    aih,
                    response_id,
                    row_index,
                    delivered_at,
                    fetched,
                ) else {
                    continue;
                };
                out.push(event);
            }
        }
    }
    Ok((out, next_after_id))
}

/// One metadata row + its fetched content → the typed event. `None`
/// when a joined boundary field the event requires is missing (torn
/// metadata — same skip policy as missing content).
fn build_event(
    msg: &MsgRow,
    agent_instance_hierarchy: String,
    response_id: String,
    row_index: i64,
    delivered_at: String,
    fetched: &Fetched,
) -> Option<AgentInstanceEvent> {
    use MessageTable as T;
    let row_sub_index = msg.row_sub_index;
    Some(match msg.table_kind {
        // ---- notifications ----
        T::MessageQueueText
        | T::MessageQueueImage
        | T::MessageQueueAudio
        | T::MessageQueueVideo
        | T::MessageQueueFile => {
            let Fetched::Media(content) = fetched else {
                return None;
            };
            AgentInstanceEvent::ClientNotificationPart {
                agent_instance_hierarchy,
                response_id,
                sender_agent_instance_hierarchy: msg
                    .sender_agent_instance_hierarchy
                    .clone()
                    .unwrap_or_default(),
                message_queue_id: msg.message_queue_id?,
                queued_at: unix_to_rfc3339(msg.timestamp_queued.unwrap_or_default()),
                key: msg.message_queue_key.clone(),
                row_index,
                part: ClientNotificationPart {
                    delivered_at,
                    content: content.clone(),
                },
            }
        }

        // ---- assistant (response side) ----
        T::AssistantResponseRefusal => AgentInstanceEvent::AssistantResponsePart {
            agent_instance_hierarchy,
            response_id,
            row_index,
            row_sub_index,
            part: AssistantResponsePart::Refusal {
                delivered_at,
                text: fetched_text(fetched)?,
            },
        },
        T::AssistantResponseReasoning => AgentInstanceEvent::AssistantResponsePart {
            agent_instance_hierarchy,
            response_id,
            row_index,
            row_sub_index,
            part: AssistantResponsePart::Reasoning {
                delivered_at,
                text: fetched_text(fetched)?,
            },
        },
        T::AssistantResponseToolCalls => AgentInstanceEvent::AssistantResponsePart {
            agent_instance_hierarchy,
            response_id,
            row_index,
            row_sub_index,
            part: tool_call_part(fetched, delivered_at, row_sub_index)?,
        },
        T::AssistantResponseContentText
        | T::AssistantResponseContentImage
        | T::AssistantResponseContentAudio
        | T::AssistantResponseContentVideo
        | T::AssistantResponseContentFile => {
            let Fetched::Media(content) = fetched else {
                return None;
            };
            AgentInstanceEvent::AssistantResponsePart {
                agent_instance_hierarchy,
                response_id,
                row_index,
                row_sub_index,
                part: assistant_media(delivered_at, content.clone()),
            }
        }

        // ---- assistant (request side) ----
        T::RequestMessageAssistantRefusal => AgentInstanceEvent::RequestMessageAssistantPart {
            agent_instance_hierarchy,
            response_id,
            row_index,
            row_sub_index,
            part: AssistantResponsePart::Refusal {
                delivered_at,
                text: fetched_text(fetched)?,
            },
        },
        T::RequestMessageAssistantReasoning => AgentInstanceEvent::RequestMessageAssistantPart {
            agent_instance_hierarchy,
            response_id,
            row_index,
            row_sub_index,
            part: AssistantResponsePart::Reasoning {
                delivered_at,
                text: fetched_text(fetched)?,
            },
        },
        T::RequestMessageAssistantToolCalls => AgentInstanceEvent::RequestMessageAssistantPart {
            agent_instance_hierarchy,
            response_id,
            row_index,
            row_sub_index,
            part: tool_call_part(fetched, delivered_at, row_sub_index)?,
        },
        T::RequestMessageAssistantContentText
        | T::RequestMessageAssistantContentImage
        | T::RequestMessageAssistantContentAudio
        | T::RequestMessageAssistantContentVideo
        | T::RequestMessageAssistantContentFile => {
            let Fetched::Media(content) = fetched else {
                return None;
            };
            AgentInstanceEvent::RequestMessageAssistantPart {
                agent_instance_hierarchy,
                response_id,
                row_index,
                row_sub_index,
                part: assistant_media(delivered_at, content.clone()),
            }
        }

        // ---- tool response (response side) ----
        T::ToolResponseContentText
        | T::ToolResponseContentImage
        | T::ToolResponseContentAudio
        | T::ToolResponseContentVideo
        | T::ToolResponseContentFile => {
            let Fetched::Media(content) = fetched else {
                return None;
            };
            AgentInstanceEvent::ToolResponsePart {
                agent_instance_hierarchy,
                response_id,
                tool_call_id: msg.tool_call_id.clone()?,
                row_index,
                row_sub_index,
                part: ToolResponsePart {
                    delivered_at,
                    content: content.clone(),
                },
            }
        }

        // ---- tool response (request side) ----
        T::RequestMessageToolContentText
        | T::RequestMessageToolContentImage
        | T::RequestMessageToolContentAudio
        | T::RequestMessageToolContentVideo
        | T::RequestMessageToolContentFile => {
            let Fetched::Media(content) = fetched else {
                return None;
            };
            AgentInstanceEvent::RequestMessageToolPart {
                agent_instance_hierarchy,
                response_id,
                tool_call_id: msg.tool_call_id.clone()?,
                row_index,
                row_sub_index,
                part: ToolResponsePart {
                    delivered_at,
                    content: content.clone(),
                },
            }
        }

        // ---- user ----
        T::RequestMessageUserContentText
        | T::RequestMessageUserContentImage
        | T::RequestMessageUserContentAudio
        | T::RequestMessageUserContentVideo
        | T::RequestMessageUserContentFile => {
            let Fetched::Media(content) = fetched else {
                return None;
            };
            AgentInstanceEvent::RequestMessageUserPart {
                agent_instance_hierarchy,
                response_id,
                row_index,
                row_sub_index,
                part: RequestMessageUserPart {
                    delivered_at,
                    content: content.clone(),
                },
            }
        }

        // ---- vector choices ----
        T::RequestVectorChoiceContentText
        | T::RequestVectorChoiceContentImage
        | T::RequestVectorChoiceContentAudio
        | T::RequestVectorChoiceContentVideo
        | T::RequestVectorChoiceContentFile => {
            let Fetched::Media(content) = fetched else {
                return None;
            };
            AgentInstanceEvent::VectorRequestChoicePart {
                agent_instance_hierarchy,
                response_id,
                key: msg.choice_key.clone()?,
                choice_index: row_index,
                part_index: row_sub_index?,
                part: VectorRequestChoicePart {
                    delivered_at,
                    content: content.clone(),
                },
            }
        }

        // Handled before dispatch / never content-built.
        T::AgentCompletionRequest
        | T::VectorCompletionRequest
        | T::FunctionExecutionRequest
        | T::ResponseVectorVote
        | T::Error => return None,
    })
}

fn fetched_text(fetched: &Fetched) -> Option<String> {
    match fetched {
        Fetched::Text(text) => Some(text.clone()),
        _ => None,
    }
}

fn tool_call_part(
    fetched: &Fetched,
    delivered_at: String,
    row_sub_index: Option<i64>,
) -> Option<AssistantResponsePart> {
    let Fetched::ToolCall {
        tool_call_id,
        function_name,
        arguments,
    } = fetched
    else {
        return None;
    };
    Some(AssistantResponsePart::ToolCall {
        delivered_at,
        function_name: function_name.clone(),
        tool_call_id: tool_call_id.clone(),
        tool_call_index: row_sub_index.unwrap_or(0),
        arguments: arguments.clone(),
    })
}
