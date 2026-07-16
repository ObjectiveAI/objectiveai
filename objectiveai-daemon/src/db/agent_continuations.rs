//! Latest-continuation registry keyed by `agent_instance_hierarchy`.
//!
//! Single function: [`upsert`]. Called by the chunk-yielder loops
//! (`agents spawn`'s `run_multi_pass` and `functions execute`'s
//! `runner::run`) before each chunk yield. The row holds whichever
//! continuation was most recently observed for that AIH; the
//! `ON CONFLICT DO UPDATE` clause keeps it idempotent against
//! repeated upserts and overwrites stale rows from prior runs.
//!
//! No read API today — this is write-only scaffolding for future
//! consumers (resume-from-continuation flows, etc.).

use super::{Error, Pool};

fn now_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Insert-or-replace the continuation for an AIH. Matches the
/// `INSERT … ON CONFLICT … DO UPDATE` idiom used by `tags::apply`.
pub async fn upsert(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    continuation: &str,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO objectiveai.agent_continuations \
             (agent_instance_hierarchy, continuation, updated_at) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (agent_instance_hierarchy) DO UPDATE SET \
             continuation = EXCLUDED.continuation, \
             updated_at   = EXCLUDED.updated_at",
    )
    .bind(agent_instance_hierarchy)
    .bind(continuation)
    .bind(now_seconds())
    .execute(&**pool)
    .await?;
    Ok(())
}
