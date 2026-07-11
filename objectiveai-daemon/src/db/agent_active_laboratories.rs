//! The laboratory ids sent with an agent's MOST RECENT spawn request,
//! backed by the postgres `agent_active_laboratories` table.
//!
//! `run_multi_pass` replaces the set wholesale after every laboratory
//! resolve (initial + each restart pass), so the table always holds
//! what the latest pass actually dialed. Most-recent-value semantics
//! (like `agent_token_usage`): the set survives deactivation and is
//! only ever superseded by the next pass. Each replace fires ONE
//! `agent_active_laboratories_changed` NOTIFY (payload = the AIH),
//! consumed by the daemon's per-agent status tracking. This is the
//! ACTIVE-laboratories concern — fully separate from the ATTACHED
//! attachments in [`super::laboratory_attachments`].

use sqlx::Row as _;

use super::{Error, Pool};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Replace the agent's active set with `ids` (in order) — one
/// transaction: delete-all, ordered inserts, one `pg_notify`. An empty
/// `ids` records "the most recent spawn request sent no laboratories".
pub async fn replace(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    ids: &[String],
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "DELETE FROM objectiveai.agent_active_laboratories \
         WHERE agent_instance_hierarchy = $1",
    )
    .bind(agent_instance_hierarchy)
    .execute(&mut *tx)
    .await?;
    let stamp = now();
    for (ordinal, id) in ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO objectiveai.agent_active_laboratories \
             (agent_instance_hierarchy, laboratory_id, ordinal, updated_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(agent_instance_hierarchy)
        .bind(id)
        .bind(ordinal as i64)
        .bind(stamp)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("SELECT pg_notify('agent_active_laboratories_changed', $1)")
        .bind(agent_instance_hierarchy)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// The agent's active set, in resolve order.
pub async fn list(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    let rows = sqlx::query(
        "SELECT laboratory_id FROM objectiveai.agent_active_laboratories \
         WHERE agent_instance_hierarchy = $1 \
         ORDER BY ordinal",
    )
    .bind(agent_instance_hierarchy)
    .fetch_all(&**pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(row.try_get::<String, _>(0)?);
    }
    Ok(out)
}
