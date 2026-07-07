//! Content-inlined conversation snapshot backing the daemon's
//! `/agents/instances/{*aih}` route.
//!
//! The same metadata query as [`super::read_all`] (shared
//! `SELECT_SHAPE` / `FROM_JOINS` / [`MsgRow`]), but instead of
//! emitting `{id, type}` parts for `agents logs read id` to resolve,
//! each row's ACTUAL content is batch-fetched from its per-kind table
//! and inlined into an SDK
//! [`ConversationRow`](objectiveai_sdk::cli::websocket_agent_instance_listener::ConversationRow)
//! — the same frame shape the live tee ships, so the WS handler
//! replays the snapshot and relays live frames through one type and
//! clients converge the seam by row identity.
//!
//! Batching: one metadata page (`"index"` ASC), then ONE query per
//! content family present in the page (`unnest` key joins /
//! `id = ANY` for the message-queue kinds) — never a per-row round
//! trip. Request-blob rows are skipped (hidden from conversations,
//! exactly as `read_all` hides them); a metadata row whose content
//! row is missing (torn write) skips that row.

use std::collections::HashMap;

use objectiveai_sdk::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};
use objectiveai_sdk::cli::websocket_agent_instance_listener::{
    ConversationRow, RowContent, RowTableKind,
};
use sqlx::Row as _;

use super::super::time::{unix_to_rfc3339, unix_to_rfc3339_opt};
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
    IndexedText { table: &'static str, refusal: bool },
    /// `(response_id, "index", tool_call_index)`-keyed tool-call table
    /// — only `arguments` is fetched (`function_name` / `tool_call_id`
    /// already ride the metadata joins).
    ToolCalls { table: &'static str },
    /// `id`-keyed per-kind message-queue content table
    /// (`row_index` = `message_queue_contents.id`).
    MessageQueue { table: &'static str, kind: ContentKind },
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
            refusal: true,
        },
        T::AssistantResponseReasoning => Source::IndexedText {
            table: "objectiveai.assistant_response_reasoning",
            refusal: false,
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
            refusal: true,
        },
        T::RequestMessageAssistantReasoning => Source::IndexedText {
            table: "objectiveai.request_message_assistant_reasoning",
            refusal: false,
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
    }
}

/// The wire kind for one metadata row. `None` for the request blobs
/// (never on the conversation stream). The three HEAD kinds never
/// appear here — heads emit no `objectiveai.messages` event; their
/// payload reaches snapshot rows via the metadata joins
/// (`tool_call_id` / `choice_key`).
fn wire_table(t: MessageTable) -> Option<RowTableKind> {
    use MessageTable as T;
    use RowTableKind as K;
    Some(match t {
        T::AgentCompletionRequest
        | T::VectorCompletionRequest
        | T::FunctionExecutionRequest => return None,
        T::MessageQueueText => K::MessageQueueText,
        T::MessageQueueImage => K::MessageQueueImage,
        T::MessageQueueAudio => K::MessageQueueAudio,
        T::MessageQueueVideo => K::MessageQueueVideo,
        T::MessageQueueFile => K::MessageQueueFile,
        T::AssistantResponseRefusal => K::AssistantResponseRefusal,
        T::AssistantResponseReasoning => K::AssistantResponseReasoning,
        T::AssistantResponseToolCalls => K::AssistantResponseToolCalls,
        T::AssistantResponseContentText => K::AssistantResponseContentText,
        T::AssistantResponseContentImage => K::AssistantResponseContentImage,
        T::AssistantResponseContentAudio => K::AssistantResponseContentAudio,
        T::AssistantResponseContentVideo => K::AssistantResponseContentVideo,
        T::AssistantResponseContentFile => K::AssistantResponseContentFile,
        T::ToolResponseContentText => K::ToolResponseContentText,
        T::ToolResponseContentImage => K::ToolResponseContentImage,
        T::ToolResponseContentAudio => K::ToolResponseContentAudio,
        T::ToolResponseContentVideo => K::ToolResponseContentVideo,
        T::ToolResponseContentFile => K::ToolResponseContentFile,
        T::RequestMessageUserContentText => K::RequestMessageUserContentText,
        T::RequestMessageUserContentImage => K::RequestMessageUserContentImage,
        T::RequestMessageUserContentAudio => K::RequestMessageUserContentAudio,
        T::RequestMessageUserContentVideo => K::RequestMessageUserContentVideo,
        T::RequestMessageUserContentFile => K::RequestMessageUserContentFile,
        T::RequestMessageAssistantRefusal => K::RequestMessageAssistantRefusal,
        T::RequestMessageAssistantReasoning => K::RequestMessageAssistantReasoning,
        T::RequestMessageAssistantToolCalls => K::RequestMessageAssistantToolCalls,
        T::RequestMessageAssistantContentText => K::RequestMessageAssistantContentText,
        T::RequestMessageAssistantContentImage => K::RequestMessageAssistantContentImage,
        T::RequestMessageAssistantContentAudio => K::RequestMessageAssistantContentAudio,
        T::RequestMessageAssistantContentVideo => K::RequestMessageAssistantContentVideo,
        T::RequestMessageAssistantContentFile => K::RequestMessageAssistantContentFile,
        T::RequestMessageToolContentText => K::RequestMessageToolContentText,
        T::RequestMessageToolContentImage => K::RequestMessageToolContentImage,
        T::RequestMessageToolContentAudio => K::RequestMessageToolContentAudio,
        T::RequestMessageToolContentVideo => K::RequestMessageToolContentVideo,
        T::RequestMessageToolContentFile => K::RequestMessageToolContentFile,
        T::RequestVectorChoiceContentText => K::RequestVectorChoiceContentText,
        T::RequestVectorChoiceContentImage => K::RequestVectorChoiceContentImage,
        T::RequestVectorChoiceContentAudio => K::RequestVectorChoiceContentAudio,
        T::RequestVectorChoiceContentVideo => K::RequestVectorChoiceContentVideo,
        T::RequestVectorChoiceContentFile => K::RequestVectorChoiceContentFile,
        T::ResponseVectorVote => K::ResponseVectorVote,
    })
}

/// Fetched content, keyed by the metadata row's
/// `(table, response_id, row_index, row_sub_index)`.
type ContentMap = HashMap<(MessageTable, String, i64, Option<i64>), RowContent>;

/// Build the per-kind media content from one fetched content row.
/// Column shapes are identical across every table of a kind — the
/// same contract [`super::read_id`] relies on.
fn media_content(kind: ContentKind, row: &sqlx::postgres::PgRow) -> Result<RowContent, Error> {
    Ok(match kind {
        ContentKind::Text => RowContent::Text {
            text: row.try_get("text")?,
        },
        ContentKind::Image => {
            let url: String = row.try_get("url")?;
            let detail_str: Option<String> = row.try_get("detail")?;
            let detail = match detail_str {
                Some(s) => serde_json::from_value(serde_json::Value::String(s))?,
                None => None,
            };
            RowContent::Image(ImageUrl { url, detail })
        }
        ContentKind::Audio => RowContent::Audio(InputAudio {
            data: row.try_get("data")?,
            format: row.try_get("format")?,
        }),
        ContentKind::Video => RowContent::Video(VideoUrl {
            url: row.try_get("url")?,
        }),
        ContentKind::File => RowContent::File(File {
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

/// One page of an agent's conversation, content inlined, in
/// `objectiveai.messages."index"` order. Returns the rows plus the
/// `after_id` cursor for the next page (`None` when this page was the
/// last). The caller (the daemon WS handler) loops pages, streaming
/// each row as one frame — bounded memory for huge histories.
pub async fn read_conversation_page(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    after_id: Option<i64>,
    limit: i64,
) -> Result<(Vec<ConversationRow>, Option<i64>), Error> {
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
    for msg in &msgs {
        match source(msg.table_kind) {
            Source::Part { .. } => {
                let (Some(index), Some(part_index)) = (msg.row_index, msg.row_sub_index) else {
                    continue;
                };
                let entry = part_keys.entry(msg.table_kind).or_default();
                entry.0.push(msg.response_id.clone());
                entry.1.push(index);
                entry.2.push(part_index);
            }
            Source::IndexedText { .. } => {
                let Some(index) = msg.row_index else { continue };
                let entry = text_keys.entry(msg.table_kind).or_default();
                entry.0.push(msg.response_id.clone());
                entry.1.push(index);
            }
            Source::ToolCalls { .. } => {
                let (Some(index), Some(tool_call_index)) = (msg.row_index, msg.row_sub_index)
                else {
                    continue;
                };
                let entry = call_keys.entry(msg.table_kind).or_default();
                entry.0.push(msg.response_id.clone());
                entry.1.push(index);
                entry.2.push(tool_call_index);
            }
            Source::MessageQueue { .. } => {
                let Some(id) = msg.row_index else { continue };
                queue_ids.entry(msg.table_kind).or_default().push(id);
            }
            Source::Vote | Source::Blob => {}
        }
    }

    let mut content: ContentMap = HashMap::new();

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
                media_content(kind, &row)?,
            );
        }
    }

    for (kind_table, (rids, indices)) in &text_keys {
        let Source::IndexedText { table, refusal } = source(*kind_table) else {
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
            let value = if refusal {
                RowContent::Refusal { text }
            } else {
                RowContent::Reasoning { text }
            };
            content.insert((*kind_table, rid, idx, None), value);
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
                RowContent::ToolCall {
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
                    (*kind_table, msg.response_id.clone(), id, None),
                    media_content(kind, &row)?,
                );
            }
        }
    }

    // Phase 3 — emit in index order, blobs skipped, content inlined.
    let mut out = Vec::with_capacity(msgs.len());
    for msg in &msgs {
        let Some(table) = wire_table(msg.table_kind) else {
            continue; // request blob — hidden
        };
        let row_content = match source(msg.table_kind) {
            Source::Vote => {
                let Some(vote) = msg
                    .vote
                    .clone()
                    .and_then(|v| serde_json::from_value(v).ok())
                else {
                    continue;
                };
                RowContent::Vote { vote }
            }
            Source::Blob => continue,
            _ => {
                // Vote rows aside, `row_index` is always Some for
                // event rows; the live tee's identity uses the same
                // value.
                let key = (
                    msg.table_kind,
                    msg.response_id.clone(),
                    msg.row_index.unwrap_or(0),
                    msg.row_sub_index,
                );
                match content.get(&key) {
                    Some(value) => value.clone(),
                    // Torn write (metadata row without its content
                    // row) — skip, exactly like a missing read_id.
                    None => continue,
                }
            }
        };
        out.push(ConversationRow {
            agent_instance_hierarchy: msg.agent_instance_hierarchy.clone(),
            response_id: msg.response_id.clone(),
            table,
            // Vote rows have NULL row_index in the DB; the live tee
            // ships 0 — match it so identities line up.
            row_index: msg.row_index.unwrap_or(0),
            row_sub_index: msg.row_sub_index,
            delivered_at: unix_to_rfc3339(msg.timestamp_delivered),
            tool_call_id: msg.tool_call_id.clone(),
            choice_key: msg.choice_key.clone(),
            sender_agent_instance_hierarchy: msg.sender_agent_instance_hierarchy.clone(),
            queued_at: unix_to_rfc3339_opt(msg.timestamp_queued),
            message_queue_key: msg.message_queue_key.clone(),
            message_queue_id: msg.message_queue_id,
            content: row_content,
        });
    }
    Ok((out, next_after_id))
}
