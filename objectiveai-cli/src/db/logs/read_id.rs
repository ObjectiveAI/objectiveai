//! `agents logs read id` backend: take a `objectiveai.messages."index"`
//! BIGSERIAL, dispatch to the per-tier target table, and build the
//! SDK `agents::logs::read::id::Response` variant with a typed
//! payload.
//!
//! Lookup is two round-trips: one to resolve the message row
//! (table discriminator + row PK fragments), one to load the
//! payload from the resolved target table. Each variant assembles
//! the SDK `Response` directly so the CLI handler stays a thin
//! pass-through.

use objectiveai_sdk::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::cli::command::agents::logs::read::id::Response;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams;
use sqlx::Row as _;

use super::super::{Error, Pool};
use super::row::MessageTable;

/// Resolve one `objectiveai.messages."index"` to the matching SDK
/// `Response` variant. Returns `Ok(None)` when no row exists at
/// that index.
pub async fn read_by_id(pool: &Pool, id: i64) -> Result<Option<Response>, Error> {
    let Some(msg) = sqlx::query(
        "SELECT response_id, \"table\" AS table_kind, row_index, row_sub_index \
         FROM objectiveai.messages \
         WHERE \"index\" = $1",
    )
    .bind(id)
    .fetch_optional(&**pool)
    .await?
    else {
        return Ok(None);
    };

    let response_id: String = msg.try_get("response_id")?;
    let table_kind: MessageTable = msg.try_get("table_kind")?;
    let row_index: Option<i64> = msg.try_get("row_index")?;
    let row_sub_index: Option<i64> = msg.try_get("row_sub_index")?;

    Ok(Some(load_payload(
        pool,
        table_kind,
        &response_id,
        row_index,
        row_sub_index,
    ).await?))
}

async fn load_payload(
    pool: &Pool,
    table_kind: MessageTable,
    response_id: &str,
    row_index: Option<i64>,
    row_sub_index: Option<i64>,
) -> Result<Response, Error> {
    match table_kind {
        // Five message_queue_* variants: `row_index` carries
        // `message_queue_contents.id`. We dispatch directly to the
        // matching per-kind table (`message_queue_texts`,
        // `_images`, etc.) and map onto the same `Text` / `Image`
        // / `Audio` / `Video` / `File` variants the assistant /
        // tool content rows use — no separate response shape.
        MessageTable::MessageQueueText => {
            let id = require_row_index(table_kind, row_index)?;
            let text: String = sqlx::query_scalar(
                "SELECT text FROM objectiveai.message_queue_texts WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&**pool)
            .await?;
            Ok(Response::Text { text })
        }
        MessageTable::MessageQueueImage => {
            let id = require_row_index(table_kind, row_index)?;
            let row = sqlx::query(
                "SELECT url, detail FROM objectiveai.message_queue_images WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&**pool)
            .await?;
            let url: String = row.try_get("url")?;
            let detail_str: Option<String> = row.try_get("detail")?;
            let detail = match detail_str {
                Some(s) => serde_json::from_value(serde_json::Value::String(s))?,
                None => None,
            };
            Ok(Response::Image(ImageUrl { url, detail }))
        }
        MessageTable::MessageQueueAudio => {
            let id = require_row_index(table_kind, row_index)?;
            let row = sqlx::query(
                "SELECT data, format FROM objectiveai.message_queue_audios WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&**pool)
            .await?;
            Ok(Response::Audio(InputAudio {
                data: row.try_get("data")?,
                format: row.try_get("format")?,
            }))
        }
        MessageTable::MessageQueueVideo => {
            let id = require_row_index(table_kind, row_index)?;
            let url: String = sqlx::query_scalar(
                "SELECT url FROM objectiveai.message_queue_videos WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&**pool)
            .await?;
            Ok(Response::Video(VideoUrl { url }))
        }
        MessageTable::MessageQueueFile => {
            let id = require_row_index(table_kind, row_index)?;
            let row = sqlx::query(
                "SELECT file_data, file_id, filename, file_url \
                 FROM objectiveai.message_queue_files WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&**pool)
            .await?;
            Ok(Response::File(File {
                file_data: row.try_get("file_data")?,
                file_id: row.try_get("file_id")?,
                filename: row.try_get("filename")?,
                file_url: row.try_get("file_url")?,
            }))
        }
        MessageTable::AgentCompletionRequest => {
            let (body, created_at, sender) =
                fetch_request_blob(pool, "objectiveai.agent_completion_requests", response_id).await?;
            let body: AgentCompletionCreateParams = serde_json::from_value(body)?;
            Ok(Response::AgentCompletionRequest {
                response_id: response_id.to_string(),
                sender_agent_instance_hierarchy: sender,
                body,
                created_at,
            })
        }
        MessageTable::VectorCompletionRequest => {
            let (body, created_at, sender) =
                fetch_request_blob(pool, "objectiveai.vector_completion_requests", response_id).await?;
            let body: VectorCompletionCreateParams = serde_json::from_value(body)?;
            Ok(Response::VectorCompletionRequest {
                response_id: response_id.to_string(),
                sender_agent_instance_hierarchy: sender,
                body,
                created_at,
            })
        }
        MessageTable::FunctionExecutionRequest => {
            let (body, created_at, sender) =
                fetch_request_blob(pool, "objectiveai.function_execution_requests", response_id).await?;
            let body: FunctionExecutionCreateParams = serde_json::from_value(body)?;
            Ok(Response::FunctionExecutionRequest {
                response_id: response_id.to_string(),
                sender_agent_instance_hierarchy: sender,
                body,
                created_at,
            })
        }
        MessageTable::ToolResponse => {
            let index = require_row_index(table_kind, row_index)?;
            let row = sqlx::query(
                "SELECT tool_call_id FROM objectiveai.tool_response \
                 WHERE response_id = $1 AND \"index\" = $2",
            )
            .bind(response_id)
            .bind(index)
            .fetch_one(&**pool)
            .await?;
            Ok(Response::ToolResponse {
                response_id: response_id.to_string(),
                index,
                tool_call_id: row.try_get("tool_call_id")?,
            })
        }
        MessageTable::AssistantResponseRefusal => {
            let index = require_row_index(table_kind, row_index)?;
            let text = fetch_indexed_text(
                pool,
                "objectiveai.assistant_response_refusal",
                response_id,
                index,
            )
            .await?;
            Ok(Response::Text { text })
        }
        MessageTable::AssistantResponseReasoning => {
            let index = require_row_index(table_kind, row_index)?;
            let text = fetch_indexed_text(
                pool,
                "objectiveai.assistant_response_reasoning",
                response_id,
                index,
            )
            .await?;
            Ok(Response::Text { text })
        }
        MessageTable::AssistantResponseToolCalls => {
            let index = require_row_index(table_kind, row_index)?;
            let tool_call_index = require_row_sub_index(table_kind, row_sub_index)?;
            let row = sqlx::query(
                "SELECT tool_call_id, function_name, arguments \
                 FROM objectiveai.assistant_response_tool_calls \
                 WHERE response_id = $1 AND \"index\" = $2 AND tool_call_index = $3",
            )
            .bind(response_id)
            .bind(index)
            .bind(tool_call_index)
            .fetch_one(&**pool)
            .await?;
            Ok(Response::ToolCall {
                response_id: response_id.to_string(),
                index,
                tool_call_index,
                tool_call_id: row.try_get("tool_call_id")?,
                function_name: row.try_get("function_name")?,
                arguments: row.try_get("arguments")?,
            })
        }
        MessageTable::AssistantResponseContentText => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let text = fetch_content_text(
                pool,
                "objectiveai.assistant_response_content_text",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::Text { text })
        }
        MessageTable::AssistantResponseContentImage => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let image_url = fetch_content_image(
                pool,
                "objectiveai.assistant_response_content_image",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::Image(image_url))
        }
        MessageTable::AssistantResponseContentAudio => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let input_audio = fetch_content_audio(
                pool,
                "objectiveai.assistant_response_content_audio",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::Audio(input_audio))
        }
        MessageTable::AssistantResponseContentVideo => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let video_url = fetch_content_video(
                pool,
                "objectiveai.assistant_response_content_video",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::Video(video_url))
        }
        MessageTable::AssistantResponseContentFile => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let file = fetch_content_file(
                pool,
                "objectiveai.assistant_response_content_file",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::File(file))
        }
        MessageTable::ToolResponseContentText => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let text = fetch_content_text(
                pool,
                "objectiveai.tool_response_content_text",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::Text { text })
        }
        MessageTable::ToolResponseContentImage => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let image_url = fetch_content_image(
                pool,
                "objectiveai.tool_response_content_image",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::Image(image_url))
        }
        MessageTable::ToolResponseContentAudio => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let input_audio = fetch_content_audio(
                pool,
                "objectiveai.tool_response_content_audio",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::Audio(input_audio))
        }
        MessageTable::ToolResponseContentVideo => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let video_url = fetch_content_video(
                pool,
                "objectiveai.tool_response_content_video",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::Video(video_url))
        }
        MessageTable::ToolResponseContentFile => {
            let (index, part_index) =
                require_full_indices(table_kind, row_index, row_sub_index)?;
            let file = fetch_content_file(
                pool,
                "objectiveai.tool_response_content_file",
                response_id,
                index,
                part_index,
            )
            .await?;
            Ok(Response::File(file))
        }
    }
}

async fn fetch_request_blob(
    pool: &Pool,
    table: &str,
    response_id: &str,
) -> Result<(serde_json::Value, i64, String), Error> {
    let sql = format!(
        "SELECT body, created_at, sender_agent_instance_hierarchy \
         FROM {table} WHERE response_id = $1",
    );
    let row = sqlx::query(&sql)
        .bind(response_id)
        .fetch_one(&**pool)
        .await?;
    let body: sqlx::types::Json<serde_json::Value> = row.try_get("body")?;
    let created_at: i64 = row.try_get("created_at")?;
    let sender: String = row.try_get("sender_agent_instance_hierarchy")?;
    Ok((body.0, created_at, sender))
}

async fn fetch_indexed_text(
    pool: &Pool,
    table: &str,
    response_id: &str,
    index: i64,
) -> Result<String, Error> {
    let sql = format!(
        "SELECT text FROM {table} \
         WHERE response_id = $1 AND \"index\" = $2",
    );
    let row = sqlx::query(&sql)
        .bind(response_id)
        .bind(index)
        .fetch_one(&**pool)
        .await?;
    Ok(row.try_get("text")?)
}

async fn fetch_content_text(
    pool: &Pool,
    table: &str,
    response_id: &str,
    index: i64,
    part_index: i64,
) -> Result<String, Error> {
    let sql = format!(
        "SELECT text FROM {table} \
         WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3",
    );
    let row = sqlx::query(&sql)
        .bind(response_id)
        .bind(index)
        .bind(part_index)
        .fetch_one(&**pool)
        .await?;
    Ok(row.try_get("text")?)
}

async fn fetch_content_image(
    pool: &Pool,
    table: &str,
    response_id: &str,
    index: i64,
    part_index: i64,
) -> Result<ImageUrl, Error> {
    let sql = format!(
        "SELECT url, detail FROM {table} \
         WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3",
    );
    let row = sqlx::query(&sql)
        .bind(response_id)
        .bind(index)
        .bind(part_index)
        .fetch_one(&**pool)
        .await?;
    let url: String = row.try_get("url")?;
    let detail_str: Option<String> = row.try_get("detail")?;
    let detail = match detail_str {
        Some(s) => serde_json::from_value(serde_json::Value::String(s))?,
        None => None,
    };
    Ok(ImageUrl { url, detail })
}

async fn fetch_content_audio(
    pool: &Pool,
    table: &str,
    response_id: &str,
    index: i64,
    part_index: i64,
) -> Result<InputAudio, Error> {
    let sql = format!(
        "SELECT data, format FROM {table} \
         WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3",
    );
    let row = sqlx::query(&sql)
        .bind(response_id)
        .bind(index)
        .bind(part_index)
        .fetch_one(&**pool)
        .await?;
    Ok(InputAudio {
        data: row.try_get("data")?,
        format: row.try_get("format")?,
    })
}

async fn fetch_content_video(
    pool: &Pool,
    table: &str,
    response_id: &str,
    index: i64,
    part_index: i64,
) -> Result<VideoUrl, Error> {
    let sql = format!(
        "SELECT url FROM {table} \
         WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3",
    );
    let row = sqlx::query(&sql)
        .bind(response_id)
        .bind(index)
        .bind(part_index)
        .fetch_one(&**pool)
        .await?;
    Ok(VideoUrl { url: row.try_get("url")? })
}

async fn fetch_content_file(
    pool: &Pool,
    table: &str,
    response_id: &str,
    index: i64,
    part_index: i64,
) -> Result<File, Error> {
    let sql = format!(
        "SELECT file_data, file_id, filename, file_url FROM {table} \
         WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3",
    );
    let row = sqlx::query(&sql)
        .bind(response_id)
        .bind(index)
        .bind(part_index)
        .fetch_one(&**pool)
        .await?;
    Ok(File {
        file_data: row.try_get("file_data")?,
        file_id: row.try_get("file_id")?,
        filename: row.try_get("filename")?,
        file_url: row.try_get("file_url")?,
    })
}

fn require_row_index(
    table_kind: MessageTable,
    row_index: Option<i64>,
) -> Result<i64, Error> {
    row_index.ok_or_else(|| {
        Error::InvalidData(format!(
            "objectiveai.messages row for table {table_kind:?} is missing row_index"
        ))
    })
}

fn require_row_sub_index(
    table_kind: MessageTable,
    row_sub_index: Option<i64>,
) -> Result<i64, Error> {
    row_sub_index.ok_or_else(|| {
        Error::InvalidData(format!(
            "objectiveai.messages row for table {table_kind:?} is missing row_sub_index"
        ))
    })
}

fn require_full_indices(
    table_kind: MessageTable,
    row_index: Option<i64>,
    row_sub_index: Option<i64>,
) -> Result<(i64, i64), Error> {
    Ok((
        require_row_index(table_kind, row_index)?,
        require_row_sub_index(table_kind, row_sub_index)?,
    ))
}
