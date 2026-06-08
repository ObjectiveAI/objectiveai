//! Per-table flat INSERT and UPDATE helpers.
//!
//! The writer's shadow already decided whether each row is new
//! (`Insert`), changed (`Update`), or unchanged (`Skip`). These
//! helpers issue the matching SQL with no `ON CONFLICT` clauses — the
//! shadow is authoritative.

use objectiveai_sdk::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};
use serde::Serialize;

use crate::db::{Error, Pool};

use super::row::RowValue;
use super::shadow::WriteOp;

/// Dispatch SQL for `value` per `op`. `Skip` is a no-op.
pub async fn write_value<'a>(pool: &Pool, op: WriteOp, value: &RowValue<'a>) -> Result<(), Error> {
    match op {
        WriteOp::Skip => Ok(()),
        WriteOp::Insert => insert_value(pool, value).await,
        WriteOp::Update => update_value(pool, value).await,
    }
}

async fn insert_value<'a>(pool: &Pool, value: &RowValue<'a>) -> Result<(), Error> {
    match *value {
        RowValue::ToolResponse { response_id, index, tool_call_id } => {
            sqlx::query(
                "INSERT INTO logs.tool_response (response_id, \"index\", tool_call_id) \
                 VALUES ($1, $2, $3)",
            )
            .bind(response_id)
            .bind(index as i64)
            .bind(tool_call_id)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseRefusal { response_id, index, text } => {
            sqlx::query(
                "INSERT INTO logs.assistant_response_refusal (response_id, \"index\", text) \
                 VALUES ($1, $2, $3)",
            )
            .bind(response_id)
            .bind(index as i64)
            .bind(text)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseReasoning { response_id, index, text } => {
            sqlx::query(
                "INSERT INTO logs.assistant_response_reasoning (response_id, \"index\", text) \
                 VALUES ($1, $2, $3)",
            )
            .bind(response_id)
            .bind(index as i64)
            .bind(text)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseToolCalls {
            response_id, index, tool_call_index, tool_call_id, arguments,
        } => {
            sqlx::query(
                "INSERT INTO logs.assistant_response_tool_calls \
                     (response_id, \"index\", tool_call_index, tool_call_id, arguments) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(response_id)
            .bind(index as i64)
            .bind(tool_call_index as i64)
            .bind(tool_call_id)
            .bind(arguments)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseContentText { response_id, index, part_index, text } => {
            insert_text_part(pool, "logs.assistant_response_content_text", response_id, index, part_index, text).await?;
        }
        RowValue::ToolResponseContentText { response_id, index, part_index, text } => {
            insert_text_part(pool, "logs.tool_response_content_text", response_id, index, part_index, text).await?;
        }
        RowValue::AssistantResponseContentImage { response_id, index, part_index, image_url } => {
            insert_image_part(pool, "logs.assistant_response_content_image", response_id, index, part_index, image_url).await?;
        }
        RowValue::ToolResponseContentImage { response_id, index, part_index, image_url } => {
            insert_image_part(pool, "logs.tool_response_content_image", response_id, index, part_index, image_url).await?;
        }
        RowValue::AssistantResponseContentAudio { response_id, index, part_index, input_audio } => {
            insert_audio_part(pool, "logs.assistant_response_content_audio", response_id, index, part_index, input_audio).await?;
        }
        RowValue::ToolResponseContentAudio { response_id, index, part_index, input_audio } => {
            insert_audio_part(pool, "logs.tool_response_content_audio", response_id, index, part_index, input_audio).await?;
        }
        RowValue::AssistantResponseContentVideo { response_id, index, part_index, video_url, is_input } => {
            insert_video_part(pool, "logs.assistant_response_content_video", response_id, index, part_index, video_url, is_input).await?;
        }
        RowValue::ToolResponseContentVideo { response_id, index, part_index, video_url, is_input } => {
            insert_video_part(pool, "logs.tool_response_content_video", response_id, index, part_index, video_url, is_input).await?;
        }
        RowValue::AssistantResponseContentFile { response_id, index, part_index, file } => {
            insert_file_part(pool, "logs.assistant_response_content_file", response_id, index, part_index, file).await?;
        }
        RowValue::ToolResponseContentFile { response_id, index, part_index, file } => {
            insert_file_part(pool, "logs.tool_response_content_file", response_id, index, part_index, file).await?;
        }
    }
    Ok(())
}

async fn update_value<'a>(pool: &Pool, value: &RowValue<'a>) -> Result<(), Error> {
    match *value {
        RowValue::ToolResponse { response_id, index, tool_call_id } => {
            sqlx::query(
                "UPDATE logs.tool_response SET tool_call_id = $3 \
                 WHERE response_id = $1 AND \"index\" = $2",
            )
            .bind(response_id)
            .bind(index as i64)
            .bind(tool_call_id)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseRefusal { response_id, index, text } => {
            sqlx::query(
                "UPDATE logs.assistant_response_refusal SET text = $3 \
                 WHERE response_id = $1 AND \"index\" = $2",
            )
            .bind(response_id)
            .bind(index as i64)
            .bind(text)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseReasoning { response_id, index, text } => {
            sqlx::query(
                "UPDATE logs.assistant_response_reasoning SET text = $3 \
                 WHERE response_id = $1 AND \"index\" = $2",
            )
            .bind(response_id)
            .bind(index as i64)
            .bind(text)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseToolCalls {
            response_id, index, tool_call_index, tool_call_id, arguments,
        } => {
            sqlx::query(
                "UPDATE logs.assistant_response_tool_calls \
                 SET tool_call_id = $4, arguments = $5 \
                 WHERE response_id = $1 AND \"index\" = $2 AND tool_call_index = $3",
            )
            .bind(response_id)
            .bind(index as i64)
            .bind(tool_call_index as i64)
            .bind(tool_call_id)
            .bind(arguments)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseContentText { response_id, index, part_index, text } => {
            update_text_part(pool, "logs.assistant_response_content_text", response_id, index, part_index, text).await?;
        }
        RowValue::ToolResponseContentText { response_id, index, part_index, text } => {
            update_text_part(pool, "logs.tool_response_content_text", response_id, index, part_index, text).await?;
        }
        RowValue::AssistantResponseContentImage { response_id, index, part_index, image_url } => {
            update_image_part(pool, "logs.assistant_response_content_image", response_id, index, part_index, image_url).await?;
        }
        RowValue::ToolResponseContentImage { response_id, index, part_index, image_url } => {
            update_image_part(pool, "logs.tool_response_content_image", response_id, index, part_index, image_url).await?;
        }
        RowValue::AssistantResponseContentAudio { response_id, index, part_index, input_audio } => {
            update_audio_part(pool, "logs.assistant_response_content_audio", response_id, index, part_index, input_audio).await?;
        }
        RowValue::ToolResponseContentAudio { response_id, index, part_index, input_audio } => {
            update_audio_part(pool, "logs.tool_response_content_audio", response_id, index, part_index, input_audio).await?;
        }
        RowValue::AssistantResponseContentVideo { response_id, index, part_index, video_url, is_input } => {
            update_video_part(pool, "logs.assistant_response_content_video", response_id, index, part_index, video_url, is_input).await?;
        }
        RowValue::ToolResponseContentVideo { response_id, index, part_index, video_url, is_input } => {
            update_video_part(pool, "logs.tool_response_content_video", response_id, index, part_index, video_url, is_input).await?;
        }
        RowValue::AssistantResponseContentFile { response_id, index, part_index, file } => {
            update_file_part(pool, "logs.assistant_response_content_file", response_id, index, part_index, file).await?;
        }
        RowValue::ToolResponseContentFile { response_id, index, part_index, file } => {
            update_file_part(pool, "logs.tool_response_content_file", response_id, index, part_index, file).await?;
        }
    }
    Ok(())
}

async fn insert_text_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, text: &str) -> Result<(), Error> {
    let sql = format!("INSERT INTO {table} (response_id, \"index\", part_index, text) VALUES ($1, $2, $3, $4)");
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64).bind(text)
        .execute(&**pool).await?;
    Ok(())
}

async fn update_text_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, text: &str) -> Result<(), Error> {
    let sql = format!("UPDATE {table} SET text = $4 WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3");
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64).bind(text)
        .execute(&**pool).await?;
    Ok(())
}

async fn insert_image_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, image: &ImageUrl) -> Result<(), Error> {
    let detail = image.detail.as_ref().and_then(|d| serde_json::to_string(d).ok());
    let sql = format!("INSERT INTO {table} (response_id, \"index\", part_index, url, detail) VALUES ($1, $2, $3, $4, $5)");
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64).bind(image.url.as_str()).bind(detail)
        .execute(&**pool).await?;
    Ok(())
}

async fn update_image_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, image: &ImageUrl) -> Result<(), Error> {
    let detail = image.detail.as_ref().and_then(|d| serde_json::to_string(d).ok());
    let sql = format!("UPDATE {table} SET url = $4, detail = $5 WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3");
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64).bind(image.url.as_str()).bind(detail)
        .execute(&**pool).await?;
    Ok(())
}

async fn insert_audio_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, audio: &InputAudio) -> Result<(), Error> {
    let sql = format!("INSERT INTO {table} (response_id, \"index\", part_index, data, format) VALUES ($1, $2, $3, $4, $5)");
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64).bind(audio.data.as_str()).bind(audio.format.as_str())
        .execute(&**pool).await?;
    Ok(())
}

async fn update_audio_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, audio: &InputAudio) -> Result<(), Error> {
    let sql = format!("UPDATE {table} SET data = $4, format = $5 WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3");
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64).bind(audio.data.as_str()).bind(audio.format.as_str())
        .execute(&**pool).await?;
    Ok(())
}

async fn insert_video_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, video: &VideoUrl, is_input: bool) -> Result<(), Error> {
    let sql = format!("INSERT INTO {table} (response_id, \"index\", part_index, url, is_input) VALUES ($1, $2, $3, $4, $5)");
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64).bind(video.url.as_str()).bind(is_input)
        .execute(&**pool).await?;
    Ok(())
}

async fn update_video_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, video: &VideoUrl, is_input: bool) -> Result<(), Error> {
    let sql = format!("UPDATE {table} SET url = $4, is_input = $5 WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3");
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64).bind(video.url.as_str()).bind(is_input)
        .execute(&**pool).await?;
    Ok(())
}

async fn insert_file_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, file: &File) -> Result<(), Error> {
    let sql = format!(
        "INSERT INTO {table} (response_id, \"index\", part_index, file_data, file_id, filename, file_url) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    );
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64)
        .bind(file.file_data.as_deref()).bind(file.file_id.as_deref()).bind(file.filename.as_deref()).bind(file.file_url.as_deref())
        .execute(&**pool).await?;
    Ok(())
}

async fn update_file_part(pool: &Pool, table: &str, response_id: &str, index: u64, part_index: u64, file: &File) -> Result<(), Error> {
    let sql = format!(
        "UPDATE {table} SET file_data = $4, file_id = $5, filename = $6, file_url = $7 \
         WHERE response_id = $1 AND \"index\" = $2 AND part_index = $3"
    );
    sqlx::query(&sql).bind(response_id).bind(index as i64).bind(part_index as i64)
        .bind(file.file_data.as_deref()).bind(file.file_id.as_deref()).bind(file.filename.as_deref()).bind(file.file_url.as_deref())
        .execute(&**pool).await?;
    Ok(())
}

// =====================================================================
// Tier blob writes
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Agent,
    Vector,
    Function,
}

impl Tier {
    pub fn request_table(self) -> &'static str {
        match self {
            Tier::Agent => "logs.agent_completion_requests",
            Tier::Vector => "logs.vector_completion_requests",
            Tier::Function => "logs.function_execution_requests",
        }
    }
    pub fn response_table(self) -> &'static str {
        match self {
            Tier::Agent => "logs.agent_completion_responses",
            Tier::Vector => "logs.vector_completion_responses",
            Tier::Function => "logs.function_execution_responses",
        }
    }
}

pub async fn insert_request_blob<P: Serialize>(
    pool: &Pool,
    tier: Tier,
    response_id: &str,
    agent_instance_hierarchy: Option<&str>,
    params: &P,
    created_at: i64,
) -> Result<(), Error> {
    let body = serde_json::to_value(params)?;
    match tier {
        Tier::Agent => {
            sqlx::query(
                "INSERT INTO logs.agent_completion_requests \
                     (response_id, agent_instance_hierarchy, body, created_at) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(response_id)
            .bind(agent_instance_hierarchy.unwrap_or(""))
            .bind(sqlx::types::Json(body))
            .bind(created_at)
            .execute(&**pool)
            .await?;
        }
        Tier::Vector | Tier::Function => {
            let sql = format!(
                "INSERT INTO {table} (response_id, body, created_at) VALUES ($1, $2, $3)",
                table = tier.request_table()
            );
            sqlx::query(&sql)
                .bind(response_id)
                .bind(sqlx::types::Json(body))
                .bind(created_at)
                .execute(&**pool)
                .await?;
        }
    }
    Ok(())
}

pub async fn insert_response_blob<C: Serialize>(
    pool: &Pool,
    tier: Tier,
    response_id: &str,
    agent_instance_hierarchy: Option<&str>,
    chunk: &C,
    created_at: i64,
) -> Result<(), Error> {
    let body = serde_json::to_value(chunk)?;
    match tier {
        Tier::Agent => {
            sqlx::query(
                "INSERT INTO logs.agent_completion_responses \
                     (response_id, agent_instance_hierarchy, body, created_at) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(response_id)
            .bind(agent_instance_hierarchy.unwrap_or(""))
            .bind(sqlx::types::Json(body))
            .bind(created_at)
            .execute(&**pool)
            .await?;
        }
        Tier::Vector | Tier::Function => {
            let sql = format!(
                "INSERT INTO {table} (response_id, body, created_at) VALUES ($1, $2, $3)",
                table = tier.response_table()
            );
            sqlx::query(&sql)
                .bind(response_id)
                .bind(sqlx::types::Json(body))
                .bind(created_at)
                .execute(&**pool)
                .await?;
        }
    }
    Ok(())
}

pub async fn update_response_blob<C: Serialize>(
    pool: &Pool,
    tier: Tier,
    response_id: &str,
    chunk: &C,
    created_at: i64,
) -> Result<(), Error> {
    let body = serde_json::to_value(chunk)?;
    let sql = format!(
        "UPDATE {table} SET body = $2, created_at = $3 WHERE response_id = $1",
        table = tier.response_table()
    );
    sqlx::query(&sql)
        .bind(response_id)
        .bind(sqlx::types::Json(body))
        .bind(created_at)
        .execute(&**pool)
        .await?;
    Ok(())
}
