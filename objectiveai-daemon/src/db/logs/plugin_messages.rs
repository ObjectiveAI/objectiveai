//! `objectiveai.plugin_messages` — captured plugin stderr.
//!
//! Written line-by-line by the `plugins run` stderr writer task (which
//! owns the child's stderr pipe); read by `plugins logs list`, filtered
//! by the plugin coordinate (owner/name/version) and paginated by the
//! BIGSERIAL `"index"` cursor. Unlike `objectiveai.messages`, this is a
//! flat single-table log — no `messages`/`messages_queue` bookkeeping.

use objectiveai_sdk::cli::command::plugins::logs::list::ResponseItem;
use sqlx::Row as _;

use super::super::{Error, Pool};

/// INSERT one captured stderr line for a `plugins run` invocation.
#[allow(clippy::too_many_arguments)]
pub async fn insert_plugin_message(
    pool: &Pool,
    owner: &str,
    name: &str,
    version: &str,
    agent_instance_hierarchy: &str,
    response_id: Option<&str>,
    created_at: i64,
    line: &str,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO objectiveai.plugin_messages \
            (owner, name, version, agent_instance_hierarchy, response_id, created_at, line) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(owner)
    .bind(name)
    .bind(version)
    .bind(agent_instance_hierarchy)
    .bind(response_id)
    .bind(created_at)
    .bind(line)
    .execute(&**pool)
    .await?;
    Ok(())
}

/// Read captured stderr lines for one plugin coordinate, ascending by
/// `"index"`, skipping rows with `"index" <= after_id` and capping at
/// `limit` (`None` = unlimited).
pub async fn read_plugin_messages(
    pool: &Pool,
    owner: &str,
    name: &str,
    version: &str,
    after_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<ResponseItem>, Error> {
    let rows = sqlx::query(
        "SELECT \"index\", owner, name, version, agent_instance_hierarchy, \
                response_id, created_at, line \
         FROM objectiveai.plugin_messages \
         WHERE owner = $1 AND name = $2 AND version = $3 \
           AND \"index\" > COALESCE($4, 0) \
         ORDER BY \"index\" ASC \
         LIMIT $5",
    )
    .bind(owner)
    .bind(name)
    .bind(version)
    .bind(after_id)
    .bind(limit)
    .fetch_all(&**pool)
    .await?;

    rows.iter()
        .map(|row| {
            Ok(ResponseItem {
                index: row.try_get("index")?,
                owner: row.try_get("owner")?,
                name: row.try_get("name")?,
                version: row.try_get("version")?,
                agent_instance_hierarchy: row.try_get("agent_instance_hierarchy")?,
                response_id: row.try_get("response_id")?,
                created_at: row.try_get("created_at")?,
                line: row.try_get("line")?,
            })
        })
        .collect()
}
