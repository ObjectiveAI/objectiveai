//! Per-table flat INSERT and UPDATE helpers — each issuing one CTE
//! that fires the streaming-content / request-blob write AND the
//! `logs.messages` / `logs.messages_queue` bookkeeping in a single
//! postgres round-trip.
//!
//! Insert path (streaming-content INSERT or request-blob INSERT):
//! ```sql
//! WITH data_ins AS (
//!     INSERT INTO logs.<table> (…) VALUES (…) RETURNING response_id
//! )
//! INSERT INTO logs.messages (response_id, "table", row_index,
//!                            row_sub_index, "index",
//!                            agent_instance_hierarchy, "timestamp")
//! SELECT $resp, $msg_table, $row_idx, $row_sub_idx,
//!        nextval('logs.messages_index_seq'),
//!        $hier, $ts
//! FROM data_ins;
//! ```
//!
//! Update path (streaming-content UPDATE):
//! ```sql
//! WITH
//!     data_upd AS (
//!         UPDATE logs.<table> SET … WHERE … RETURNING response_id
//!     ),
//!     msg AS (
//!         SELECT "index" AS msg_index FROM logs.messages
//!         WHERE response_id = $resp AND "table" = $msg_table
//!           AND row_index IS NOT DISTINCT FROM $row_idx
//!           AND row_sub_index IS NOT DISTINCT FROM $row_sub_idx
//!     )
//! UPDATE logs.messages_queue
//! SET read_index = msg.msg_index - 1
//! FROM msg, data_upd
//! WHERE spawned_agent_instance_hierarchy = $hier
//!   AND read_index >= msg.msg_index;
//! ```
//!
//! Response-blob writes (the three `_responses` tables) DON'T touch
//! `logs.messages` — they're not events, just the latest snapshot.

use objectiveai_sdk::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};
use serde::Serialize;

use crate::db::{Error, Pool};

use super::row::{MessageTable, RowValue};
use super::shadow::WriteOp;

/// Dispatch SQL for `value` per `op`. `Skip` is a no-op.
pub async fn write_value<'a>(
    pool: &Pool,
    op: WriteOp,
    value: &RowValue<'a>,
    timestamp: i64,
) -> Result<(), Error> {
    match op {
        WriteOp::Skip => Ok(()),
        WriteOp::Insert => insert_value(pool, value, timestamp).await,
        WriteOp::Update => update_value(pool, value).await,
    }
}

async fn insert_value<'a>(
    pool: &Pool,
    value: &RowValue<'a>,
    timestamp: i64,
) -> Result<(), Error> {
    let mt = value.message_table();
    let hier = value.agent_instance_hierarchy();
    let row_index = value.row_index();
    let row_sub_index = value.row_sub_index();
    let response_id = value.response_id();

    match *value {
        RowValue::ToolResponse { tool_call_id, .. } => {
            sqlx::query(
                "WITH data_ins AS (\
                    INSERT INTO logs.tool_response (response_id, \"index\", tool_call_id) \
                    VALUES ($1, $2, $3) RETURNING response_id\
                 )\
                 INSERT INTO logs.messages \
                    (response_id, \"table\", row_index, row_sub_index, \
                     agent_instance_hierarchy, \"timestamp\") \
                 SELECT $1, $4, $5, $6, $7, $8 FROM data_ins",
            )
            .bind(response_id)
            .bind(row_index)
            .bind(tool_call_id)
            .bind(mt)
            .bind(row_index)
            .bind(row_sub_index)
            .bind(hier)
            .bind(timestamp)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseRefusal { text, .. } => {
            sqlx::query(
                "WITH data_ins AS (\
                    INSERT INTO logs.assistant_response_refusal (response_id, \"index\", text) \
                    VALUES ($1, $2, $3) RETURNING response_id\
                 )\
                 INSERT INTO logs.messages \
                    (response_id, \"table\", row_index, row_sub_index, \
                     agent_instance_hierarchy, \"timestamp\") \
                 SELECT $1, $4, $5, $6, $7, $8 FROM data_ins",
            )
            .bind(response_id)
            .bind(row_index)
            .bind(text)
            .bind(mt)
            .bind(row_index)
            .bind(row_sub_index)
            .bind(hier)
            .bind(timestamp)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseReasoning { text, .. } => {
            sqlx::query(
                "WITH data_ins AS (\
                    INSERT INTO logs.assistant_response_reasoning (response_id, \"index\", text) \
                    VALUES ($1, $2, $3) RETURNING response_id\
                 )\
                 INSERT INTO logs.messages \
                    (response_id, \"table\", row_index, row_sub_index, \
                     agent_instance_hierarchy, \"timestamp\") \
                 SELECT $1, $4, $5, $6, $7, $8 FROM data_ins",
            )
            .bind(response_id)
            .bind(row_index)
            .bind(text)
            .bind(mt)
            .bind(row_index)
            .bind(row_sub_index)
            .bind(hier)
            .bind(timestamp)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseToolCalls {
            tool_call_index, tool_call_id, arguments, ..
        } => {
            sqlx::query(
                "WITH data_ins AS (\
                    INSERT INTO logs.assistant_response_tool_calls \
                        (response_id, \"index\", tool_call_index, tool_call_id, arguments) \
                    VALUES ($1, $2, $3, $4, $5) RETURNING response_id\
                 )\
                 INSERT INTO logs.messages \
                    (response_id, \"table\", row_index, row_sub_index, \
                     agent_instance_hierarchy, \"timestamp\") \
                 SELECT $1, $6, $7, $8, $9, $10 FROM data_ins",
            )
            .bind(response_id)
            .bind(row_index)
            .bind(tool_call_index as i64)
            .bind(tool_call_id)
            .bind(arguments)
            .bind(mt)
            .bind(row_index)
            .bind(row_sub_index)
            .bind(hier)
            .bind(timestamp)
            .execute(&**pool)
            .await?;
        }
        RowValue::AssistantResponseContentText { text, .. } => {
            insert_text_part_with_msg(pool, "logs.assistant_response_content_text", value, text, timestamp).await?;
        }
        RowValue::ToolResponseContentText { text, .. } => {
            insert_text_part_with_msg(pool, "logs.tool_response_content_text", value, text, timestamp).await?;
        }
        RowValue::AssistantResponseContentImage { image_url, .. } => {
            insert_image_part_with_msg(pool, "logs.assistant_response_content_image", value, image_url, timestamp).await?;
        }
        RowValue::ToolResponseContentImage { image_url, .. } => {
            insert_image_part_with_msg(pool, "logs.tool_response_content_image", value, image_url, timestamp).await?;
        }
        RowValue::AssistantResponseContentAudio { input_audio, .. } => {
            insert_audio_part_with_msg(pool, "logs.assistant_response_content_audio", value, input_audio, timestamp).await?;
        }
        RowValue::ToolResponseContentAudio { input_audio, .. } => {
            insert_audio_part_with_msg(pool, "logs.tool_response_content_audio", value, input_audio, timestamp).await?;
        }
        RowValue::AssistantResponseContentVideo { video_url, is_input, .. } => {
            insert_video_part_with_msg(pool, "logs.assistant_response_content_video", value, video_url, is_input, timestamp).await?;
        }
        RowValue::ToolResponseContentVideo { video_url, is_input, .. } => {
            insert_video_part_with_msg(pool, "logs.tool_response_content_video", value, video_url, is_input, timestamp).await?;
        }
        RowValue::AssistantResponseContentFile { file, .. } => {
            insert_file_part_with_msg(pool, "logs.assistant_response_content_file", value, file, timestamp).await?;
        }
        RowValue::ToolResponseContentFile { file, .. } => {
            insert_file_part_with_msg(pool, "logs.tool_response_content_file", value, file, timestamp).await?;
        }
    }
    Ok(())
}

async fn update_value<'a>(pool: &Pool, value: &RowValue<'a>) -> Result<(), Error> {
    let mt = value.message_table();
    let hier = value.agent_instance_hierarchy();
    let row_index = value.row_index();
    let row_sub_index = value.row_sub_index();
    let response_id = value.response_id();

    match *value {
        RowValue::ToolResponse { tool_call_id, .. } => {
            run_update_with_downgrade(
                pool,
                "UPDATE logs.tool_response SET tool_call_id = $A \
                 WHERE response_id = $RESP AND \"index\" = $RI",
                response_id, row_index, row_sub_index, mt, hier,
                &[("A", BindVal::Str(tool_call_id))],
                &[BindIdx::Resp, BindIdx::Ri],
            ).await?;
        }
        RowValue::AssistantResponseRefusal { text, .. } => {
            run_update_with_downgrade(
                pool,
                "UPDATE logs.assistant_response_refusal SET text = $A \
                 WHERE response_id = $RESP AND \"index\" = $RI",
                response_id, row_index, row_sub_index, mt, hier,
                &[("A", BindVal::Str(text))],
                &[BindIdx::Resp, BindIdx::Ri],
            ).await?;
        }
        RowValue::AssistantResponseReasoning { text, .. } => {
            run_update_with_downgrade(
                pool,
                "UPDATE logs.assistant_response_reasoning SET text = $A \
                 WHERE response_id = $RESP AND \"index\" = $RI",
                response_id, row_index, row_sub_index, mt, hier,
                &[("A", BindVal::Str(text))],
                &[BindIdx::Resp, BindIdx::Ri],
            ).await?;
        }
        RowValue::AssistantResponseToolCalls { tool_call_index, tool_call_id, arguments, .. } => {
            run_update_with_downgrade(
                pool,
                "UPDATE logs.assistant_response_tool_calls SET tool_call_id = $A, arguments = $B \
                 WHERE response_id = $RESP AND \"index\" = $RI AND tool_call_index = $RSI",
                response_id, row_index, row_sub_index, mt, hier,
                &[("A", BindVal::Str(tool_call_id)), ("B", BindVal::Str(arguments))],
                &[BindIdx::Resp, BindIdx::Ri, BindIdx::Rsi],
            ).await?;
            let _ = tool_call_index;
        }
        RowValue::AssistantResponseContentText { text, .. }
        | RowValue::ToolResponseContentText { text, .. } => {
            let table = match *value {
                RowValue::AssistantResponseContentText { .. } => "logs.assistant_response_content_text",
                _ => "logs.tool_response_content_text",
            };
            let sql = format!(
                "UPDATE {table} SET text = $A \
                 WHERE response_id = $RESP AND \"index\" = $RI AND part_index = $RSI"
            );
            run_update_with_downgrade(
                pool, &sql,
                response_id, row_index, row_sub_index, mt, hier,
                &[("A", BindVal::Str(text))],
                &[BindIdx::Resp, BindIdx::Ri, BindIdx::Rsi],
            ).await?;
        }
        RowValue::AssistantResponseContentImage { image_url, .. }
        | RowValue::ToolResponseContentImage { image_url, .. } => {
            let table = match *value {
                RowValue::AssistantResponseContentImage { .. } => "logs.assistant_response_content_image",
                _ => "logs.tool_response_content_image",
            };
            let detail = image_url.detail.as_ref().and_then(|d| serde_json::to_string(d).ok());
            let sql = format!(
                "UPDATE {table} SET url = $A, detail = $B \
                 WHERE response_id = $RESP AND \"index\" = $RI AND part_index = $RSI"
            );
            run_update_with_downgrade(
                pool, &sql,
                response_id, row_index, row_sub_index, mt, hier,
                &[("A", BindVal::Str(image_url.url.as_str())), ("B", BindVal::OptString(detail))],
                &[BindIdx::Resp, BindIdx::Ri, BindIdx::Rsi],
            ).await?;
        }
        RowValue::AssistantResponseContentAudio { input_audio, .. }
        | RowValue::ToolResponseContentAudio { input_audio, .. } => {
            let table = match *value {
                RowValue::AssistantResponseContentAudio { .. } => "logs.assistant_response_content_audio",
                _ => "logs.tool_response_content_audio",
            };
            let sql = format!(
                "UPDATE {table} SET data = $A, format = $B \
                 WHERE response_id = $RESP AND \"index\" = $RI AND part_index = $RSI"
            );
            run_update_with_downgrade(
                pool, &sql,
                response_id, row_index, row_sub_index, mt, hier,
                &[
                    ("A", BindVal::Str(input_audio.data.as_str())),
                    ("B", BindVal::Str(input_audio.format.as_str())),
                ],
                &[BindIdx::Resp, BindIdx::Ri, BindIdx::Rsi],
            ).await?;
        }
        RowValue::AssistantResponseContentVideo { video_url, is_input, .. }
        | RowValue::ToolResponseContentVideo { video_url, is_input, .. } => {
            let table = match *value {
                RowValue::AssistantResponseContentVideo { .. } => "logs.assistant_response_content_video",
                _ => "logs.tool_response_content_video",
            };
            let sql = format!(
                "UPDATE {table} SET url = $A, is_input = $B \
                 WHERE response_id = $RESP AND \"index\" = $RI AND part_index = $RSI"
            );
            run_update_with_downgrade(
                pool, &sql,
                response_id, row_index, row_sub_index, mt, hier,
                &[
                    ("A", BindVal::Str(video_url.url.as_str())),
                    ("B", BindVal::Bool(is_input)),
                ],
                &[BindIdx::Resp, BindIdx::Ri, BindIdx::Rsi],
            ).await?;
        }
        RowValue::AssistantResponseContentFile { file, .. }
        | RowValue::ToolResponseContentFile { file, .. } => {
            let table = match *value {
                RowValue::AssistantResponseContentFile { .. } => "logs.assistant_response_content_file",
                _ => "logs.tool_response_content_file",
            };
            let sql = format!(
                "UPDATE {table} SET file_data = $A, file_id = $B, filename = $C, file_url = $D \
                 WHERE response_id = $RESP AND \"index\" = $RI AND part_index = $RSI"
            );
            run_update_with_downgrade(
                pool, &sql,
                response_id, row_index, row_sub_index, mt, hier,
                &[
                    ("A", BindVal::OptStr(file.file_data.as_deref())),
                    ("B", BindVal::OptStr(file.file_id.as_deref())),
                    ("C", BindVal::OptStr(file.filename.as_deref())),
                    ("D", BindVal::OptStr(file.file_url.as_deref())),
                ],
                &[BindIdx::Resp, BindIdx::Ri, BindIdx::Rsi],
            ).await?;
        }
    }
    Ok(())
}

// ---- INSERT helpers for content parts (shared CTE shape) -------------

async fn insert_text_part_with_msg<'a>(
    pool: &Pool,
    table: &str,
    value: &RowValue<'a>,
    text: &str,
    timestamp: i64,
) -> Result<(), Error> {
    let sql = format!(
        "WITH data_ins AS (\
            INSERT INTO {table} (response_id, \"index\", part_index, text) \
            VALUES ($1, $2, $3, $4) RETURNING response_id\
         )\
         INSERT INTO logs.messages \
            (response_id, \"table\", row_index, row_sub_index, \
             agent_instance_hierarchy, \"timestamp\") \
         SELECT $1, $5, $6, $7, $8, $9 FROM data_ins"
    );
    sqlx::query(&sql)
        .bind(value.response_id())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(text)
        .bind(value.message_table())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(value.agent_instance_hierarchy())
        .bind(timestamp)
        .execute(&**pool)
        .await?;
    Ok(())
}

async fn insert_image_part_with_msg<'a>(
    pool: &Pool,
    table: &str,
    value: &RowValue<'a>,
    image: &ImageUrl,
    timestamp: i64,
) -> Result<(), Error> {
    let detail = image.detail.as_ref().and_then(|d| serde_json::to_string(d).ok());
    let sql = format!(
        "WITH data_ins AS (\
            INSERT INTO {table} (response_id, \"index\", part_index, url, detail) \
            VALUES ($1, $2, $3, $4, $5) RETURNING response_id\
         )\
         INSERT INTO logs.messages \
            (response_id, \"table\", row_index, row_sub_index, \
             agent_instance_hierarchy, \"timestamp\") \
         SELECT $1, $6, $7, $8, $9, $10 FROM data_ins"
    );
    sqlx::query(&sql)
        .bind(value.response_id())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(image.url.as_str())
        .bind(detail)
        .bind(value.message_table())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(value.agent_instance_hierarchy())
        .bind(timestamp)
        .execute(&**pool)
        .await?;
    Ok(())
}

async fn insert_audio_part_with_msg<'a>(
    pool: &Pool,
    table: &str,
    value: &RowValue<'a>,
    audio: &InputAudio,
    timestamp: i64,
) -> Result<(), Error> {
    let sql = format!(
        "WITH data_ins AS (\
            INSERT INTO {table} (response_id, \"index\", part_index, data, format) \
            VALUES ($1, $2, $3, $4, $5) RETURNING response_id\
         )\
         INSERT INTO logs.messages \
            (response_id, \"table\", row_index, row_sub_index, \
             agent_instance_hierarchy, \"timestamp\") \
         SELECT $1, $6, $7, $8, $9, $10 FROM data_ins"
    );
    sqlx::query(&sql)
        .bind(value.response_id())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(audio.data.as_str())
        .bind(audio.format.as_str())
        .bind(value.message_table())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(value.agent_instance_hierarchy())
        .bind(timestamp)
        .execute(&**pool)
        .await?;
    Ok(())
}

async fn insert_video_part_with_msg<'a>(
    pool: &Pool,
    table: &str,
    value: &RowValue<'a>,
    video: &VideoUrl,
    is_input: bool,
    timestamp: i64,
) -> Result<(), Error> {
    let sql = format!(
        "WITH data_ins AS (\
            INSERT INTO {table} (response_id, \"index\", part_index, url, is_input) \
            VALUES ($1, $2, $3, $4, $5) RETURNING response_id\
         )\
         INSERT INTO logs.messages \
            (response_id, \"table\", row_index, row_sub_index, \
             agent_instance_hierarchy, \"timestamp\") \
         SELECT $1, $6, $7, $8, $9, $10 FROM data_ins"
    );
    sqlx::query(&sql)
        .bind(value.response_id())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(video.url.as_str())
        .bind(is_input)
        .bind(value.message_table())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(value.agent_instance_hierarchy())
        .bind(timestamp)
        .execute(&**pool)
        .await?;
    Ok(())
}

async fn insert_file_part_with_msg<'a>(
    pool: &Pool,
    table: &str,
    value: &RowValue<'a>,
    file: &File,
    timestamp: i64,
) -> Result<(), Error> {
    let sql = format!(
        "WITH data_ins AS (\
            INSERT INTO {table} (response_id, \"index\", part_index, file_data, file_id, filename, file_url) \
            VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING response_id\
         )\
         INSERT INTO logs.messages \
            (response_id, \"table\", row_index, row_sub_index, \
             agent_instance_hierarchy, \"timestamp\") \
         SELECT $1, $8, $9, $10, $11, $12 FROM data_ins"
    );
    sqlx::query(&sql)
        .bind(value.response_id())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(file.file_data.as_deref())
        .bind(file.file_id.as_deref())
        .bind(file.filename.as_deref())
        .bind(file.file_url.as_deref())
        .bind(value.message_table())
        .bind(value.row_index())
        .bind(value.row_sub_index())
        .bind(value.agent_instance_hierarchy())
        .bind(timestamp)
        .execute(&**pool)
        .await?;
    Ok(())
}

// ---- UPDATE helper: streaming row + messages_queue downgrade -----------
//
// The update is parameterized on a placeholder SQL template that uses
// named tokens for the response_id / row_index / row_sub_index binds
// plus arbitrary per-table value binds (A, B, C, D). The helper
// rewrites the tokens into positional $N placeholders and appends the
// messages-queue downgrade CTE.

#[derive(Clone, Copy)]
enum BindIdx {
    Resp,
    Ri,
    Rsi,
}

enum BindVal<'a> {
    Str(&'a str),
    OptStr(Option<&'a str>),
    OptString(Option<String>),
    Bool(bool),
}

#[allow(clippy::too_many_arguments)]
async fn run_update_with_downgrade<'a>(
    pool: &Pool,
    update_sql_template: &str,
    response_id: &str,
    row_index: i64,
    row_sub_index: Option<i64>,
    message_table: MessageTable,
    agent_instance_hierarchy: &str,
    extra_binds: &[(&str, BindVal<'a>)],
    update_where_binds: &[BindIdx],
) -> Result<(), Error> {
    // Assign positional indices. Order in the final SQL:
    //   $1..$N  = update_where_binds in order, then extra_binds in
    //             declaration order. (We rewrite the template's
    //             named tokens accordingly.)
    //   $(N+1)  = message_table
    //   $(N+2)  = row_index
    //   $(N+3)  = row_sub_index
    //   $(N+4)  = agent_instance_hierarchy
    let mut sql = update_sql_template.to_string();
    let mut pos = 1usize;

    // Replace each WHERE bind token with its positional index. The
    // resp/ri/rsi positions inside `sql` are written back into the
    // template via `sql.replace(...)`; we only need to remember
    // resp_pos for the downgrade CTE below.
    let mut resp_pos: Option<usize> = None;
    for slot in update_where_binds {
        let idx = pos;
        pos += 1;
        match slot {
            BindIdx::Resp => {
                sql = sql.replace("$RESP", &format!("${idx}"));
                resp_pos = Some(idx);
            }
            BindIdx::Ri => {
                sql = sql.replace("$RI", &format!("${idx}"));
            }
            BindIdx::Rsi => {
                sql = sql.replace("$RSI", &format!("${idx}"));
            }
        }
    }
    let resp_pos = resp_pos.expect("Resp bind required");

    // Replace extra-bind tokens ($A, $B, $C, $D) with positional.
    for (token, _val) in extra_binds {
        let idx = pos;
        pos += 1;
        sql = sql.replace(&format!("${token}"), &format!("${idx}"));
    }

    let mt_pos = pos; pos += 1;
    let ri_for_msg_pos = pos; pos += 1;
    let rsi_for_msg_pos = pos; pos += 1;
    let hier_pos = pos;

    let final_sql = format!(
        "WITH \
            data_upd AS ({sql} RETURNING response_id),\
            msg AS (\
                SELECT \"index\" AS msg_index FROM logs.messages \
                WHERE response_id = ${resp_pos} \
                  AND \"table\" = ${mt_pos} \
                  AND row_index IS NOT DISTINCT FROM ${ri_for_msg_pos} \
                  AND row_sub_index IS NOT DISTINCT FROM ${rsi_for_msg_pos}\
            )\
         UPDATE logs.messages_queue \
         SET read_index = msg.msg_index - 1 \
         FROM msg, data_upd \
         WHERE spawned_agent_instance_hierarchy = ${hier_pos} \
           AND read_index >= msg.msg_index",
    );

    let mut q = sqlx::query(&final_sql);
    // Bind WHERE clause values in their declared order.
    for slot in update_where_binds {
        q = match slot {
            BindIdx::Resp => q.bind(response_id),
            BindIdx::Ri => q.bind(row_index),
            BindIdx::Rsi => q.bind(row_sub_index),
        };
    }
    // Bind extra values.
    for (_, val) in extra_binds {
        q = match val {
            BindVal::Str(s) => q.bind(*s),
            BindVal::OptStr(s) => q.bind(*s),
            BindVal::OptString(s) => q.bind(s.clone()),
            BindVal::Bool(b) => q.bind(*b),
        };
    }
    // Bind messages-row identification + agent hierarchy.
    q = q.bind(message_table);
    q = q.bind(row_index);
    q = q.bind(row_sub_index);
    q = q.bind(agent_instance_hierarchy);

    q.execute(&**pool).await?;
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
    /// The matching [`MessageTable`] for this tier's request blob.
    /// Response blobs don't emit messages so there's no equivalent.
    pub fn request_message_table(self) -> MessageTable {
        match self {
            Tier::Agent => MessageTable::AgentCompletionRequest,
            Tier::Vector => MessageTable::VectorCompletionRequest,
            Tier::Function => MessageTable::FunctionExecutionRequest,
        }
    }
}

/// INSERT the request blob. Called once per stream, on first chunk
/// arrival. Request blobs don't carry `agent_instance_hierarchy` —
/// they're shared across every agent that participates in the stream.
/// The per-agent "the request was made for me" linkage lives in
/// `logs.messages` and is written separately by
/// [`insert_request_messages_row`] the first time each agent appears
/// in the chunk's row iterator.
pub async fn insert_request_blob<P: Serialize>(
    pool: &Pool,
    tier: Tier,
    response_id: &str,
    params: &P,
    timestamp: i64,
) -> Result<(), Error> {
    let body = serde_json::to_value(params)?;
    let sql = format!(
        "INSERT INTO {table} (response_id, body, created_at) VALUES ($1, $2, $3)",
        table = tier.request_table()
    );
    sqlx::query(&sql)
        .bind(response_id)
        .bind(sqlx::types::Json(body))
        .bind(timestamp)
        .execute(&**pool)
        .await?;
    Ok(())
}

/// INSERT a `logs.messages` row that registers this stream's request
/// blob in the agent's history. Called once per (stream, agent) pair
/// — the writer tracks which agents it has already seen and only
/// emits this row the first time it encounters a new one in the row
/// iterator. By postgres's BIGSERIAL `"index"` assignment, this row
/// is guaranteed to land earlier in the agent's history than any
/// subsequent streaming-content row that the same writer call
/// sequences after it.
pub async fn insert_request_messages_row(
    pool: &Pool,
    tier: Tier,
    response_id: &str,
    agent_instance_hierarchy: &str,
    timestamp: i64,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO logs.messages \
            (response_id, \"table\", row_index, row_sub_index, \
             agent_instance_hierarchy, \"timestamp\") \
         VALUES ($1, $2, NULL, NULL, $3, $4)",
    )
    .bind(response_id)
    .bind(tier.request_message_table())
    .bind(agent_instance_hierarchy)
    .bind(timestamp)
    .execute(&**pool)
    .await?;
    Ok(())
}

/// INSERT the response tier blob (first tick only). Response blobs
/// don't emit messages — they're the latest snapshot, not events.
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

/// UPDATE the response tier blob (subsequent ticks).
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
