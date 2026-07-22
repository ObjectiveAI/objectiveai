//! Task writes: create, delete, the scheduler's atomic claim /
//! completion, and the boot reconcile. Every state transition is a
//! single atomic UPDATE — the WHERE clauses are the whole concurrency
//! story (see `schema.sql`).

use objectiveai_sdk::cli::command::AgentArguments;
use sqlx::Row as _;

use super::super::{Error, Pool};
use super::ClaimedTask;

/// Insert a freshly-created task, armed at `now + delay_secs`.
#[allow(clippy::too_many_arguments)]
pub async fn insert_task(
    pool: &Pool,
    id: &str,
    command: &serde_json::Value,
    agent_arguments: &AgentArguments,
    delay_secs: i64,
    repeat: bool,
    repeat_count: Option<i64>,
) -> Result<(), Error> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO objectiveai.tasks \
         (id, command, agent_arguments, delay_secs, repeat, repeat_count, \
          state, created_at, next_run_at) \
         VALUES ($1, $2, $3, $4, $5, $6, 'scheduled', $7, $8)",
    )
    .bind(id)
    .bind(sqlx::types::Json(command))
    .bind(sqlx::types::Json(agent_arguments))
    .bind(delay_secs)
    .bind(repeat)
    .bind(repeat_count)
    .bind(now)
    .bind(now + delay_secs)
    .execute(&**pool)
    .await?;
    Ok(())
}

/// Delete a task. `true` when a row was actually removed. A task
/// deleted mid-run finishes its in-flight run, whose completion
/// UPDATE then matches nothing (no resurrection).
pub async fn delete_task(pool: &Pool, id: &str) -> Result<bool, Error> {
    let result = sqlx::query("DELETE FROM objectiveai.tasks WHERE id = $1")
        .bind(id)
        .execute(&**pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// The scheduler's FIRE claim: atomically move every DUE task
/// (`scheduled`, `next_run_at <= now`) to `running` and return what
/// each fire needs. The single UPDATE is what makes a fire exclusive
/// across concurrent schedulers.
pub async fn claim_due(pool: &Pool, now: i64) -> Result<Vec<ClaimedTask>, Error> {
    let rows = sqlx::query(
        "UPDATE objectiveai.tasks SET state = 'running' \
         WHERE state = 'scheduled' AND next_run_at <= $1 \
         RETURNING id, command, agent_arguments",
    )
    .bind(now)
    .fetch_all(&**pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            let sqlx::types::Json(command) = row.try_get("command")?;
            let sqlx::types::Json(agent_arguments) = row.try_get("agent_arguments")?;
            Ok(ClaimedTask {
                id: row.try_get("id")?,
                command,
                agent_arguments,
            })
        })
        .collect()
}

/// A fired run's COMPLETION: one atomic UPDATE writing the counters,
/// the last result, and the next state — `complete` for a one-shot
/// (any outcome) or a counted repeat whose SUCCESS budget
/// (`run_count - error_count`) is now met; otherwise re-armed at
/// `now + delay_secs`. Errored runs never consume the budget.
/// Matches nothing when the task was deleted mid-run — by design.
pub async fn complete_run(
    pool: &Pool,
    id: &str,
    errored: bool,
    now: i64,
) -> Result<(), Error> {
    let err: i64 = if errored { 1 } else { 0 };
    sqlx::query(
        "UPDATE objectiveai.tasks SET \
           run_count = run_count + 1, \
           error_count = error_count + $2, \
           last_result = $3, \
           state = CASE \
             WHEN NOT repeat THEN 'complete' \
             WHEN repeat_count IS NOT NULL \
               AND (run_count + 1 - (error_count + $2)) >= repeat_count \
               THEN 'complete' \
             ELSE 'scheduled' \
           END, \
           next_run_at = CASE \
             WHEN NOT repeat THEN NULL \
             WHEN repeat_count IS NOT NULL \
               AND (run_count + 1 - (error_count + $2)) >= repeat_count \
               THEN NULL \
             ELSE $4 + delay_secs \
           END \
         WHERE id = $1 AND state = 'running'",
    )
    .bind(id)
    .bind(err)
    .bind(if errored { "error" } else { "success" })
    .bind(now)
    .execute(&**pool)
    .await?;
    Ok(())
}

/// Boot reconcile: rows stranded in `running` are a crashed daemon's
/// orphans — re-arm them due IMMEDIATELY (at-least-once; the lost run
/// was never counted).
pub async fn reconcile_running(pool: &Pool, now: i64) -> Result<(), Error> {
    sqlx::query(
        "UPDATE objectiveai.tasks SET state = 'scheduled', next_run_at = $1 \
         WHERE state = 'running'",
    )
    .bind(now)
    .execute(&**pool)
    .await?;
    Ok(())
}
