//! Logged failures — the `error` log kind.
//!
//! Errors are written by this dedicated helper, NOT the chunk-driven
//! [`writer`](super::writer) (no dedup/update semantics — an error is
//! one immutable row). One CTE round-trip inserts the
//! `objectiveai.errors` content row and its `objectiveai.messages`
//! event row; the content row's BIGSERIAL `id` doubles as the
//! messages `row_index`, so identity never collides even when
//! `response_id` is NULL (the post-lock pre-stream case — the only
//! kind allowed a NULL response_id).

use crate::db::{Error, Pool};

use super::row::MessageTable;

/// Persist one error into an agent's history. `response_id` is the
/// response the failure belongs to when one existed, `None` for
/// post-lock pre-stream failures. `error` is the CLI's user-facing
/// error value (`Error::output_message()`). Returns the
/// `objectiveai.errors.id` — the row's `messages.row_index`.
pub async fn insert_error(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    response_id: Option<&str>,
    error: &serde_json::Value,
    timestamp: i64,
) -> Result<i64, Error> {
    let (id,): (i64,) = sqlx::query_as(
        "WITH data_ins AS (\
            INSERT INTO objectiveai.errors \
                (agent_instance_hierarchy, response_id, error) \
            VALUES ($1, $2, $3) RETURNING id\
         ), msg_ins AS (\
            INSERT INTO objectiveai.messages \
                (response_id, \"table\", row_index, row_sub_index, \
                 agent_instance_hierarchy, \"timestamp\") \
            SELECT $2, $4, id, NULL, $1, $5 FROM data_ins\
         ) \
         SELECT id FROM data_ins",
    )
    .bind(agent_instance_hierarchy)
    .bind(response_id)
    .bind(error)
    .bind(MessageTable::Error)
    .bind(timestamp)
    .fetch_one(&**pool)
    .await?;
    Ok(id)
}
