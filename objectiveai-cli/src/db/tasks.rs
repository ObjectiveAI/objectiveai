//! `schedules` table + the `agents tasks {schedule, list, run}` storage
//! tier.
//!
//! Per-row payload: an argv vector to invoke on each scheduled poll,
//! the minimum interval between invocations in seconds, and a JSON
//! snapshot of the caller's `AgentArguments` so the runner can
//! re-install identity env vars at fire-time.

use objectiveai_sdk::cli::command::AgentArguments;
use sqlx::Row as _;

use super::{Error, Pool};

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
    pub last_ran_at: Option<i64>,
    pub interval_seconds: Option<u64>,
}

/// Subset of a schedule row that `agents tasks run` needs to fire one
/// task. Each row here has already had its `last_ran_at` bumped to
/// `now`, and (if it was a oneshot) been deleted, inside the same
/// transaction.
#[derive(Debug, Clone)]
pub struct RunRow {
    pub id: i64,
    pub name: String,
    pub command: Vec<String>,
    pub agent_arguments: AgentArguments,
}

fn now_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Insert one schedule row and return its auto-incremented id.
///
/// `command` is JSON-serialised as a string array (the argv shape the
/// runner will exec). `agent_arguments` is JSON-serialised verbatim —
/// the runner re-installs each `Some(_)` field as the matching env var
/// when the schedule fires.
pub async fn insert_schedule(
    pool: &Pool,
    name: &str,
    command: &[String],
    description: &str,
    agent_instance_hierarchy: &str,
    interval_seconds: Option<u64>,
    agent_arguments: &AgentArguments,
) -> Result<i64, Error> {
    let command_json = serde_json::to_string(command)?;
    let agent_arguments_json = serde_json::to_string(agent_arguments)?;
    let interval_param: Option<i64> = interval_seconds.map(|s| s as i64);
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO schedules \
         (name, command, description, agent_instance_hierarchy, interval_seconds, agent_arguments, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING id",
    )
    .bind(name)
    .bind(command_json)
    .bind(description)
    .bind(agent_instance_hierarchy)
    .bind(interval_param)
    .bind(agent_arguments_json)
    .bind(now_seconds())
    .fetch_one(&**pool)
    .await?;
    Ok(id)
}

/// List `schedules` matching the supplied filters. Every filter is
/// optional and composes additively — the SQL is one statement that
/// gates each predicate on whether the corresponding bind is active
/// (0 = inactive bool flag, `NULL` = unset depth/count).
///
/// * `parent` + `max_depth`: hierarchy scope.
/// * `oneshot_only` / `interval_only`: kind filter (mutually exclusive
///   at the CLI layer; both `false` = no kind filter).
/// * `pending_only` / `exhausted_only`: readiness filter (same).
/// * `offset` / `count`: pagination. `count = None` binds `-1` to
///   LIMIT for unlimited (postgres treats negative LIMIT as
///   unlimited via `LIMIT NULL`; we pass NULL explicitly).
pub async fn list_schedules(
    pool: &Pool,
    parent: &str,
    max_depth: Option<u64>,
    oneshot_only: bool,
    interval_only: bool,
    pending_only: bool,
    exhausted_only: bool,
    offset: u64,
    count: Option<u64>,
) -> Result<Vec<ListedSchedule>, Error> {
    let max_depth_param: Option<i64> = max_depth.map(|d| d as i64);
    let count_param: Option<i64> = count.map(|c| c as i64);
    let offset_param: i64 = offset as i64;

    // Positional binds in the same order as `$1..$9`. The SQL mirrors
    // the sqlite predecessor's named_params version 1:1 modulo
    // postgres dialect (`length(replace(x, '/', ''))` works identically;
    // the `($N IS NULL OR …)` short-circuits are postgres-native).
    let rows = sqlx::query(
        "SELECT id, \
                name, \
                agent_instance_hierarchy, \
                command, \
                description, \
                created_at, \
                last_ran_at, \
                interval_seconds \
         FROM schedules \
         WHERE \
             /* Hierarchy + depth filter. Inclusive of parent itself. */ \
             ( \
                 agent_instance_hierarchy = $1 \
                 OR ( \
                     agent_instance_hierarchy LIKE ($1 || '/%') \
                     AND ( \
                         $2::bigint IS NULL \
                         OR \
                         ( \
                             (length(agent_instance_hierarchy) \
                              - length(replace(agent_instance_hierarchy, '/', ''))) \
                             - (length($1) \
                                - length(replace($1, '/', ''))) \
                         ) <= $2 \
                     ) \
                 ) \
             ) \
             AND ($3 = 0 OR interval_seconds IS NULL) \
             AND ($4 = 0 OR interval_seconds IS NOT NULL) \
             AND ($5 = 0 OR ( \
                 (interval_seconds IS NULL AND last_ran_at IS NULL) \
                 OR \
                 (interval_seconds IS NOT NULL \
                  AND (last_ran_at IS NULL \
                       OR ($6 - last_ran_at) >= interval_seconds)) \
             )) \
             AND ($7 = 0 OR ( \
                 (interval_seconds IS NULL AND last_ran_at IS NOT NULL) \
                 OR \
                 (interval_seconds IS NOT NULL \
                  AND last_ran_at IS NOT NULL \
                  AND ($6 - last_ran_at) < interval_seconds) \
             )) \
         ORDER BY id ASC \
         LIMIT $8 OFFSET $9",
    )
    .bind(parent)
    .bind(max_depth_param)
    .bind(oneshot_only as i64)
    .bind(interval_only as i64)
    .bind(pending_only as i64)
    .bind(now_seconds())
    .bind(exhausted_only as i64)
    .bind(count_param)
    .bind(offset_param)
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
        });
    }
    Ok(out)
}

/// Atomically: capture every pending row in scope, bump every captured
/// row's `last_ran_at = now`, and delete any captured oneshots.
/// Returns the captured rows for the caller to fire.
///
/// "Pending" means: oneshots with `last_ran_at IS NULL`, or recurring
/// rows where `now - last_ran_at >= interval_seconds` (or
/// `last_ran_at IS NULL`). Same predicate `agents tasks list --pending`
/// matches.
pub async fn collect_and_mark_pending(
    pool: &Pool,
    parent: &str,
    max_depth: Option<u64>,
) -> Result<Vec<RunRow>, Error> {
    let now = now_seconds();
    let max_depth_param: Option<i64> = max_depth.map(|d| d as i64);

    let mut tx = pool.begin().await?;

    // 1. Capture rows.
    let rows = sqlx::query(
        "SELECT id, name, command, agent_arguments \
         FROM schedules \
         WHERE \
             ( \
                 agent_instance_hierarchy = $1 \
                 OR ( \
                     agent_instance_hierarchy LIKE ($1 || '/%') \
                     AND ( \
                         $2::bigint IS NULL \
                         OR \
                         ( \
                             (length(agent_instance_hierarchy) \
                              - length(replace(agent_instance_hierarchy, '/', ''))) \
                             - (length($1) \
                                - length(replace($1, '/', ''))) \
                         ) <= $2 \
                     ) \
                 ) \
             ) \
             AND ( \
                 (interval_seconds IS NULL AND last_ran_at IS NULL) \
                 OR \
                 (interval_seconds IS NOT NULL \
                  AND (last_ran_at IS NULL \
                       OR ($3 - last_ran_at) >= interval_seconds)) \
             ) \
         ORDER BY id ASC",
    )
    .bind(parent)
    .bind(max_depth_param)
    .bind(now)
    .fetch_all(&mut *tx)
    .await?;

    if rows.is_empty() {
        tx.commit().await?;
        return Ok(Vec::new());
    }

    // 2. Bump last_ran_at + 3. Delete oneshots. Loop per-id since the
    //    delete predicate is also per-row.
    let mut captured: Vec<(i64, String, String, String)> = Vec::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get(0)?;
        let name: String = row.try_get(1)?;
        let command_json: String = row.try_get(2)?;
        let agent_arguments_json: String = row.try_get(3)?;
        captured.push((id, name, command_json, agent_arguments_json));
    }
    for (id, _, _, _) in &captured {
        sqlx::query("UPDATE schedules SET last_ran_at = $1 WHERE id = $2")
            .bind(now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "DELETE FROM schedules WHERE id = $1 AND interval_seconds IS NULL",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // 4. Decode and return.
    let mut out = Vec::with_capacity(captured.len());
    for (id, name, command_json, agent_arguments_json) in captured {
        let command: Vec<String> = serde_json::from_str(&command_json)?;
        let agent_arguments: AgentArguments = serde_json::from_str(&agent_arguments_json)?;
        out.push(RunRow {
            id,
            name,
            command,
            agent_arguments,
        });
    }
    Ok(out)
}
