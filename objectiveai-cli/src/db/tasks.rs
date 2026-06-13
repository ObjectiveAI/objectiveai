//! `schedules` + `tasks_runs` + `tasks_logs` — the `agents tasks
//! {schedule, list, run}` storage tier.
//!
//! Per-schedule payload: an argv vector to invoke on each scheduled
//! poll, the minimum interval between invocations in seconds, and a
//! JSON snapshot of the caller's `AgentArguments` so the runner can
//! re-install identity env vars at fire-time.
//!
//! Schedule rows are NEVER deleted. Versions are separate rows (`--
//! overwrite` inserts `version = max+1`, shadowing the older rows);
//! run history is one `tasks_runs` row per firing (written atomically
//! by [`claim_pending`]'s claim query); each item a fired task emits
//! is one `tasks_logs` row linked to its run.

use objectiveai_sdk::cli::command::AgentArguments;
use sqlx::Row as _;

use super::{Error, Pool};

/// Serializes every [`claim_pending`] claim transaction via
/// `pg_advisory_xact_lock` — readiness reads `tasks_runs` while a
/// concurrent claimer's inserts aren't yet visible, so row locks alone
/// can't prevent a double-claim.
const CLAIM_LOCK_KEY: i64 = 0x7461_736b_735f_7275; // "tasks_ru"

/// SQL predicate: this `schedules` row (aliased `s`) is the newest
/// version of its `(name, agent_instance_hierarchy)` — older versions
/// are shadowed: never listed, never run.
const LATEST_VERSION_PREDICATE: &str = "s.version = ( \
    SELECT MAX(s2.version) FROM objectiveai.schedules s2 \
    WHERE s2.name = s.name \
      AND s2.agent_instance_hierarchy = s.agent_instance_hierarchy \
)";

/// One row from `schedules` as surfaced by `agents tasks list`.
/// `command` is decoded from its JSON-string column.
#[derive(Debug, Clone)]
pub struct ListedSchedule {
    pub id: i64,
    pub name: String,
    pub agent_instance_hierarchy: String,
    pub command: Vec<String>,
    pub description: String,
    pub created_at: i64,
    /// Unix seconds of this row's newest `tasks_runs` entry — derived,
    /// not a column.
    pub last_ran_at: Option<i64>,
    pub interval_seconds: Option<u64>,
    /// Version of this row: `1` on first insert, `max+1` per
    /// `--overwrite`. Only the newest version of a `(name, aih)` lists.
    pub version: i64,
    /// The plugin that registered this schedule, if any. `Some` iff all
    /// three `plugin_*` columns are set (the table CHECK enforces it).
    pub plugin: Option<crate::plugin_path::PluginPath>,
}

/// Subset of a schedule row that `agents tasks run` needs to fire one
/// task, plus the id of the `tasks_runs` row the claim minted for this
/// firing (the log writer links every emitted item to it).
#[derive(Debug, Clone)]
pub struct RunRow {
    pub run_id: i64,
    pub name: String,
    pub agent_instance_hierarchy: String,
    pub version: i64,
    pub command: Vec<String>,
    pub agent_arguments: AgentArguments,
    /// The plugin that registered this schedule, if any — re-installed
    /// on the run ctx so the task fires on behalf of that plugin.
    pub plugin: Option<crate::plugin_path::PluginPath>,
}

fn now_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Insert one schedule row and return its `(id, version)`.
///
/// `command` is JSON-serialised as a string array (the argv shape the
/// runner will exec). `agent_arguments` is JSON-serialised verbatim —
/// the runner re-installs each `Some(_)` field as the matching env var
/// when the schedule fires.
///
/// Versions are separate rows, unique on `(name, aih, version)`:
/// * `overwrite = false` inserts `version = 1`; if the schedule was
///   ever created, version 1 already exists and the unique violation
///   yields `Ok(None)` so the caller can raise a friendly "already
///   exists" error.
/// * `overwrite = true` inserts a NEW row with `version = max + 1`,
///   shadowing the older rows (they never list or run again, but stay
///   for per-version run history). The fresh row has no `tasks_runs`
///   entries, so it is immediately pending. A concurrent overwrite can
///   collide on the computed version; retry a bounded number of times.
pub async fn insert_schedule(
    pool: &Pool,
    name: &str,
    command: &[String],
    description: &str,
    agent_instance_hierarchy: &str,
    interval_seconds: Option<u64>,
    agent_arguments: &AgentArguments,
    plugin: Option<&crate::plugin_path::PluginPath>,
    overwrite: bool,
) -> Result<Option<(i64, i64)>, Error> {
    let command_json = serde_json::to_string(command)?;
    let agent_arguments_json = serde_json::to_string(agent_arguments)?;
    let interval_param: Option<i64> = interval_seconds.map(|s| s as i64);
    let (plugin_owner, plugin_repository, plugin_version) = match plugin {
        Some(p) => (
            Some(p.owner.as_str()),
            Some(p.repository.as_str()),
            Some(p.version.as_str()),
        ),
        None => (None, None, None),
    };

    let columns = "(name, command, description, agent_instance_hierarchy, interval_seconds, \
                    agent_arguments, plugin_owner, plugin_repository, plugin_version, created_at, \
                    version)";
    // Non-overwrite always claims version 1 — colliding with it means
    // the schedule already exists (rows are never deleted). Overwrite
    // computes max+1 in the INSERT itself.
    let version_expr = if overwrite {
        "(SELECT COALESCE(MAX(version), 0) + 1 FROM objectiveai.schedules \
          WHERE name = $1 AND agent_instance_hierarchy = $4)"
    } else {
        "1"
    };
    let query = format!(
        "INSERT INTO objectiveai.schedules {columns} \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, {version_expr}) \
         RETURNING id, version"
    );

    let mut attempts = 0;
    loop {
        attempts += 1;
        let result = sqlx::query_as::<_, (i64, i64)>(&query)
            .bind(name)
            .bind(&command_json)
            .bind(description)
            .bind(agent_instance_hierarchy)
            .bind(interval_param)
            .bind(&agent_arguments_json)
            .bind(plugin_owner)
            .bind(plugin_repository)
            .bind(plugin_version)
            .bind(now_seconds())
            .fetch_one(&**pool)
            .await;

        return match result {
            Ok(pair) => Ok(Some(pair)),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                if overwrite {
                    if attempts < 3 {
                        // Concurrent overwrite computed the same max+1
                        // — re-run; MAX re-evaluates against the
                        // winner.
                        continue;
                    }
                    // Still colliding after retries — surface the db
                    // error rather than a misleading "already exists"
                    // (the caller DID pass --overwrite).
                    Err(sqlx::Error::Database(e).into())
                } else {
                    // Collision on (name, aih, 1): the schedule already
                    // exists — report absence so the handler can
                    // surface a clean error.
                    Ok(None)
                }
            }
            Err(e) => Err(e.into()),
        };
    }
}

/// List `schedules` matching the supplied filters. Only the newest
/// version of each `(name, aih)` is visible; readiness derives from
/// each row's newest `tasks_runs` entry. Every filter is optional and
/// composes additively — the SQL is one statement that gates each
/// predicate on whether the corresponding bind is active (0 = inactive
/// bool flag, `NULL` = unset after_id/count).
///
/// * `hierarchies`: exact-match AIH scope (no subtree descent).
/// * `oneshot_only` / `interval_only`: kind filter (mutually exclusive
///   at the CLI layer; both `false` = no kind filter).
/// * `pending_only` / `exhausted_only`: readiness filter (same).
/// * `after_id` / `count`: keyset pagination by ascending row id.
///   `count = None` binds `NULL` to LIMIT for unlimited.
pub async fn list_schedules(
    pool: &Pool,
    hierarchies: &[String],
    oneshot_only: bool,
    interval_only: bool,
    pending_only: bool,
    exhausted_only: bool,
    after_id: Option<i64>,
    count: Option<u64>,
) -> Result<Vec<ListedSchedule>, Error> {
    let count_param: Option<i64> = count.map(|c| c as i64);

    // Exact-AIH scope (`= ANY($1)`, no subtree descent), only the
    // newest version of each (name, aih) (older versions are
    // shadowed), `last_ran_at` derived from the newest `tasks_runs`
    // entry via LATERAL, the kind / readiness short-circuits
    // (`($N = 0 OR …)`, $5 = now), and keyset pagination forward by
    // ascending id (`id > COALESCE($7, 0)`). `LIMIT $8` is `NULL` when
    // `count` is `None` → unlimited.
    let query = format!(
        "SELECT s.id, \
                s.name, \
                s.agent_instance_hierarchy, \
                s.command, \
                s.description, \
                s.created_at, \
                lr.last_ran_at, \
                s.interval_seconds, \
                s.plugin_owner, \
                s.plugin_repository, \
                s.plugin_version, \
                s.version \
         FROM objectiveai.schedules s \
         LEFT JOIN LATERAL ( \
             SELECT MAX(r.ran_at) AS last_ran_at \
             FROM objectiveai.tasks_runs r WHERE r.schedule_id = s.id \
         ) lr ON TRUE \
         WHERE s.agent_instance_hierarchy = ANY($1) \
             AND {LATEST_VERSION_PREDICATE} \
             AND ($2 = 0 OR s.interval_seconds IS NULL) \
             AND ($3 = 0 OR s.interval_seconds IS NOT NULL) \
             AND ($4 = 0 OR ( \
                 (s.interval_seconds IS NULL AND lr.last_ran_at IS NULL) \
                 OR \
                 (s.interval_seconds IS NOT NULL \
                  AND (lr.last_ran_at IS NULL \
                       OR ($5 - lr.last_ran_at) >= s.interval_seconds)) \
             )) \
             AND ($6 = 0 OR ( \
                 (s.interval_seconds IS NULL AND lr.last_ran_at IS NOT NULL) \
                 OR \
                 (s.interval_seconds IS NOT NULL \
                  AND lr.last_ran_at IS NOT NULL \
                  AND ($5 - lr.last_ran_at) < s.interval_seconds) \
             )) \
             AND s.id > COALESCE($7, 0) \
         ORDER BY s.id ASC \
         LIMIT $8",
    );
    let rows = sqlx::query(&query)
    .bind(hierarchies)
    .bind(oneshot_only as i64)
    .bind(interval_only as i64)
    .bind(pending_only as i64)
    .bind(now_seconds())
    .bind(exhausted_only as i64)
    .bind(after_id)
    .bind(count_param)
    .fetch_all(&**pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get(0)?;
        let name: String = row.try_get(1)?;
        let agent_instance_hierarchy: String = row.try_get(2)?;
        let command_json: String = row.try_get(3)?;
        let description: String = row.try_get(4)?;
        let created_at: i64 = row.try_get(5)?;
        let last_ran_at: Option<i64> = row.try_get(6)?;
        let interval_seconds: Option<i64> = row.try_get(7)?;
        let plugin_owner: Option<String> = row.try_get(8)?;
        let plugin_repository: Option<String> = row.try_get(9)?;
        let plugin_version: Option<String> = row.try_get(10)?;
        let version: i64 = row.try_get(11)?;
        let command: Vec<String> = serde_json::from_str(&command_json)?;
        out.push(ListedSchedule {
            id,
            name,
            agent_instance_hierarchy,
            command,
            description,
            created_at,
            last_ran_at,
            interval_seconds: interval_seconds.map(|s| s as u64),
            version,
            plugin: crate::plugin_path::PluginPath::from_parts(
                plugin_owner,
                plugin_repository,
                plugin_version,
            ),
        });
    }
    Ok(out)
}

/// Atomically claim every pending schedule in `parent`'s subtree (its
/// own AIH or any descendant): ONE statement both selects the eligible
/// rows and inserts their `tasks_runs` rows, so each returned `RunRow`
/// carries the freshly-minted `run_id`. Schedules are never updated or
/// deleted by a run — exhaustion is purely "has a run" (oneshots) /
/// "newest run too recent" (recurring), and only the newest version of
/// each (name, aih) is eligible.
///
/// The transaction opens with `pg_advisory_xact_lock` so concurrent
/// `tasks run` callers fully serialize: the second claimer sees the
/// first's `tasks_runs` rows and excludes those schedules. (Row locks
/// can't do this — eligibility reads a DIFFERENT table than the one
/// the claim writes, so a blocked reader would re-check a schedules
/// row that never changed and double-claim.)
pub async fn claim_pending(pool: &Pool, parent: &str) -> Result<Vec<RunRow>, Error> {
    let now = now_seconds();

    let mut tx = pool.begin().await?;

    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(CLAIM_LOCK_KEY)
        .execute(&mut *tx)
        .await?;

    let query = format!(
        "WITH eligible AS ( \
             SELECT s.id, s.name, s.agent_instance_hierarchy, s.version, \
                    s.command, s.agent_arguments, \
                    s.plugin_owner, s.plugin_repository, s.plugin_version \
             FROM objectiveai.schedules s \
             WHERE ( \
                     s.agent_instance_hierarchy = $1 \
                     OR s.agent_instance_hierarchy LIKE ($1 || '/%') \
                 ) \
                 AND {LATEST_VERSION_PREDICATE} \
                 AND ( \
                     (s.interval_seconds IS NULL \
                      AND NOT EXISTS ( \
                          SELECT 1 FROM objectiveai.tasks_runs r WHERE r.schedule_id = s.id \
                      )) \
                     OR \
                     (s.interval_seconds IS NOT NULL \
                      AND COALESCE( \
                          $2 - (SELECT MAX(r.ran_at) FROM objectiveai.tasks_runs r \
                                WHERE r.schedule_id = s.id) \
                              >= s.interval_seconds, \
                          TRUE)) \
                 ) \
         ), \
         ins AS ( \
             INSERT INTO objectiveai.tasks_runs (schedule_id, ran_at) \
             SELECT id, $2 FROM eligible \
             RETURNING id AS run_id, schedule_id \
         ) \
         SELECT ins.run_id, e.name, e.agent_instance_hierarchy, e.version, \
                e.command, e.agent_arguments, \
                e.plugin_owner, e.plugin_repository, e.plugin_version \
         FROM eligible e \
         JOIN ins ON ins.schedule_id = e.id \
         ORDER BY e.id ASC",
    );
    let rows = sqlx::query(&query)
        .bind(parent)
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let run_id: i64 = row.try_get(0)?;
        let name: String = row.try_get(1)?;
        let agent_instance_hierarchy: String = row.try_get(2)?;
        let version: i64 = row.try_get(3)?;
        let command_json: String = row.try_get(4)?;
        let agent_arguments_json: String = row.try_get(5)?;
        let plugin_owner: Option<String> = row.try_get(6)?;
        let plugin_repository: Option<String> = row.try_get(7)?;
        let plugin_version: Option<String> = row.try_get(8)?;
        let command: Vec<String> = serde_json::from_str(&command_json)?;
        let agent_arguments: AgentArguments = serde_json::from_str(&agent_arguments_json)?;
        out.push(RunRow {
            run_id,
            name,
            agent_instance_hierarchy,
            version,
            command,
            agent_arguments,
            plugin: crate::plugin_path::PluginPath::from_parts(
                plugin_owner,
                plugin_repository,
                plugin_version,
            ),
        });
    }
    Ok(out)
}

/// Append one emitted item to a run's log. `value` is the serialized
/// `tasks::run::ResponseItem` exactly as it crossed the wire.
pub async fn insert_task_log(pool: &Pool, run_id: i64, value: &str) -> Result<(), Error> {
    sqlx::query("INSERT INTO objectiveai.tasks_logs (run_id, value, created_at) VALUES ($1, $2, $3)")
        .bind(run_id)
        .bind(value)
        .bind(now_seconds())
        .execute(&**pool)
        .await?;
    Ok(())
}
