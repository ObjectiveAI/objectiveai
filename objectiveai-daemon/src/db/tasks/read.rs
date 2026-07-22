//! Task reads: the full listing and the scheduler's next-wakeup scan.

use sqlx::Row as _;

use super::super::{Error, Pool};
use super::TaskRow;

/// Every task, ascending by creation time (completed tasks included —
/// they stay listed until deleted).
pub async fn list_tasks(pool: &Pool) -> Result<Vec<TaskRow>, Error> {
    let rows = sqlx::query(
        "SELECT id, command, agent_arguments, delay_secs, repeat, \
                repeat_count, run_count, error_count, last_result, \
                state, created_at, next_run_at \
         FROM objectiveai.tasks ORDER BY created_at, id",
    )
    .fetch_all(&**pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            let sqlx::types::Json(command) = row.try_get("command")?;
            let sqlx::types::Json(agent_arguments) =
                row.try_get("agent_arguments")?;
            let state: String = row.try_get("state")?;
            Ok(TaskRow {
                id: row.try_get("id")?,
                command,
                agent_arguments,
                delay_secs: row.try_get("delay_secs")?,
                repeat: row.try_get("repeat")?,
                repeat_count: row.try_get("repeat_count")?,
                run_count: row.try_get("run_count")?,
                error_count: row.try_get("error_count")?,
                last_result: row.try_get("last_result")?,
                complete: state == "complete",
                created_at: row.try_get("created_at")?,
                next_run_at: row.try_get("next_run_at")?,
            })
        })
        .collect()
}

/// The earliest armed fire time — the scheduler's sleep target.
/// `None` = nothing scheduled (park until notified).
pub async fn next_due(pool: &Pool) -> Result<Option<i64>, Error> {
    let row = sqlx::query(
        "SELECT MIN(next_run_at) AS next FROM objectiveai.tasks \
         WHERE state = 'scheduled'",
    )
    .fetch_one(&**pool)
    .await?;
    Ok(row.try_get("next")?)
}
