//! Task writes: create, delete, the scheduler's atomic claim /
//! completion, and the boot reconcile. Every state transition is a
//! single atomic UPDATE — the WHERE clauses are the whole concurrency
//! story (see `schema.sql`).

use objectiveai_sdk::cli::command::AgentArguments;
use sqlx::Row as _;

use super::super::{Error, Pool};
use super::ClaimedTask;

/// Insert a freshly-created task, armed at `now + delay_secs`. `id`
/// is the user-chosen name whose NAMESPACE is the creator's plugin
/// trio (taken from `agent_arguments` — plain identity, not
/// authentication); `(trio, id)` is the task's sole identity.
/// `Ok(false)` = that pair is already taken (unique violation);
/// `Ok(true)` = inserted.
pub async fn insert_task(
    pool: &Pool,
    id: &str,
    command: &serde_json::Value,
    agent_arguments: &AgentArguments,
    delay_secs: i64,
    repeat: bool,
    repeat_count: Option<i64>,
) -> Result<bool, Error> {
    let now = chrono::Utc::now().timestamp();
    let result = sqlx::query(
        "INSERT INTO objectiveai.tasks \
         (id, plugin_owner, plugin_name, plugin_version, \
          agent_instance_hierarchy, agent_id, agent_full_id, agent_remote, \
          response_id, response_ids, \
          command, delay_secs, repeat, repeat_count, \
          state, created_at, next_run_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, \
                 'scheduled', $15, $16)",
    )
    .bind(id)
    .bind(agent_arguments.plugin_owner.as_deref())
    .bind(agent_arguments.plugin_name.as_deref())
    .bind(agent_arguments.plugin_version.as_deref())
    // The AIH is non-null in the schema; scope_identity always sets
    // it, but default defensively rather than fail the insert.
    .bind(agent_arguments.agent_instance_hierarchy.as_deref().unwrap_or("unknown"))
    .bind(agent_arguments.agent_id.as_deref())
    .bind(agent_arguments.agent_full_id.as_deref())
    .bind(agent_arguments.agent_remote.as_deref())
    .bind(agent_arguments.response_id.as_deref())
    .bind(agent_arguments.response_ids.as_deref())
    .bind(sqlx::types::Json(command))
    .bind(delay_secs)
    .bind(repeat)
    .bind(repeat_count)
    .bind(now)
    .bind(now + delay_secs)
    .execute(&**pool)
    .await;
    match result {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(db)) if db.is_unique_violation() => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Delete a task by its user-facing id, resolved within the caller's
/// plugin namespace — that's simply what the id MEANS (identity, not
/// authentication); `IS NOT DISTINCT FROM` makes the all-NULL
/// non-plugin namespace match too. `true` when a row was actually
/// removed. A task deleted mid-run finishes its in-flight run, whose
/// completion UPDATE then matches nothing (no resurrection).
pub async fn delete_task(
    pool: &Pool,
    plugin: (Option<&str>, Option<&str>, Option<&str>),
    id: &str,
) -> Result<bool, Error> {
    let result = sqlx::query(
        "DELETE FROM objectiveai.tasks \
         WHERE id = $1 \
           AND plugin_owner IS NOT DISTINCT FROM $2 \
           AND plugin_name IS NOT DISTINCT FROM $3 \
           AND plugin_version IS NOT DISTINCT FROM $4",
    )
    .bind(id)
    .bind(plugin.0)
    .bind(plugin.1)
    .bind(plugin.2)
    .execute(&**pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// The scheduler's FIRE claim: atomically move every DUE task
/// (`scheduled`, `next_run_at <= now`) to `running` and return what
/// each fire needs. The single UPDATE is what makes a fire exclusive
/// across concurrent schedulers.
pub async fn claim_due(pool: &Pool, now: i64) -> Result<Vec<ClaimedTask>, Error> {
    let rows = sqlx::query(&format!(
        "UPDATE objectiveai.tasks SET state = 'running' \
         WHERE state = 'scheduled' AND next_run_at <= $1 \
         RETURNING id, command, {}",
        super::IDENTITY_COLUMNS,
    ))
    .bind(now)
    .fetch_all(&**pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            let sqlx::types::Json(command) = row.try_get("command")?;
            let agent_arguments = super::identity_from_row(&row)?;
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
/// `now + delay_secs`. Targets the task by its sole identity —
/// `(plugin trio, id)`, matched with IS NOT DISTINCT FROM. Matches
/// nothing when the task was deleted mid-run — by design.
pub async fn complete_run(
    pool: &Pool,
    plugin: (Option<&str>, Option<&str>, Option<&str>),
    id: &str,
    errored: bool,
    now: i64,
) -> Result<(), Error> {
    let err: i64 = if errored { 1 } else { 0 };
    sqlx::query(
        "UPDATE objectiveai.tasks SET \
           run_count = run_count + 1, \
           error_count = error_count + $5, \
           last_result = $6, \
           state = CASE \
             WHEN NOT repeat THEN 'complete' \
             WHEN repeat_count IS NOT NULL \
               AND (run_count + 1 - (error_count + $5)) >= repeat_count \
               THEN 'complete' \
             ELSE 'scheduled' \
           END, \
           next_run_at = CASE \
             WHEN NOT repeat THEN NULL \
             WHEN repeat_count IS NOT NULL \
               AND (run_count + 1 - (error_count + $5)) >= repeat_count \
               THEN NULL \
             ELSE $7 + delay_secs \
           END \
         WHERE id = $1 \
           AND plugin_owner IS NOT DISTINCT FROM $2 \
           AND plugin_name IS NOT DISTINCT FROM $3 \
           AND plugin_version IS NOT DISTINCT FROM $4 \
           AND state = 'running'",
    )
    .bind(id)
    .bind(plugin.0)
    .bind(plugin.1)
    .bind(plugin.2)
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
