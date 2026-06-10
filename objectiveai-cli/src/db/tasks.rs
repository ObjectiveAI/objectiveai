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
    /// Overwrite count: `1` on first insert, incremented on each
    /// `--overwrite` replacement of this `(name, aih)` row.
    pub version: i64,
    /// The plugin that registered this schedule, if any. `Some` iff all
    /// three `plugin_*` columns are set (the table CHECK enforces it).
    pub plugin: Option<crate::plugin_path::PluginPath>,
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
///
/// `(name, agent_instance_hierarchy)` is unique. When `overwrite` is
/// false a collision yields `Ok(None)` so the caller can raise a
/// friendly "already exists" error; when true the existing row is
/// replaced in place and its `version` bumped (`last_ran_at` reset so
/// the redefined schedule fires fresh). A brand-new row starts at
/// `version = 1`.
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
) -> Result<Option<i64>, Error> {
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

    // `version` is omitted from the column list so the table's
    // `DEFAULT 1` applies on a fresh insert; the overwrite path bumps
    // it explicitly off the existing row.
    let columns = "(name, command, description, agent_instance_hierarchy, interval_seconds, \
                    agent_arguments, plugin_owner, plugin_repository, plugin_version, created_at)";
    let values = "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";

    let query = if overwrite {
        format!(
            "INSERT INTO schedules {columns} {values} \
             ON CONFLICT (name, agent_instance_hierarchy) DO UPDATE SET \
                 command = EXCLUDED.command, \
                 description = EXCLUDED.description, \
                 interval_seconds = EXCLUDED.interval_seconds, \
                 agent_arguments = EXCLUDED.agent_arguments, \
                 plugin_owner = EXCLUDED.plugin_owner, \
                 plugin_repository = EXCLUDED.plugin_repository, \
                 plugin_version = EXCLUDED.plugin_version, \
                 last_ran_at = NULL, \
                 version = schedules.version + 1 \
             RETURNING id"
        )
    } else {
        format!("INSERT INTO schedules {columns} {values} RETURNING id")
    };

    let result = sqlx::query_scalar::<_, i64>(&query)
        .bind(name)
        .bind(command_json)
        .bind(description)
        .bind(agent_instance_hierarchy)
        .bind(interval_param)
        .bind(agent_arguments_json)
        .bind(plugin_owner)
        .bind(plugin_repository)
        .bind(plugin_version)
        .bind(now_seconds())
        .fetch_one(&**pool)
        .await;

    match result {
        Ok(id) => Ok(Some(id)),
        // Non-overwrite collision on the (name, aih) unique constraint:
        // report absence so the handler can surface a clean error.
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Ok(None),
        Err(e) => Err(e.into()),
    }
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
    hierarchies: &[String],
    oneshot_only: bool,
    interval_only: bool,
    pending_only: bool,
    exhausted_only: bool,
    after_id: Option<i64>,
    count: Option<u64>,
) -> Result<Vec<ListedSchedule>, Error> {
    let count_param: Option<i64> = count.map(|c| c as i64);

    // Exact-AIH scope (`= ANY($1)`, no subtree descent), the kind /
    // readiness short-circuits (`($N = 0 OR …)`, $5 = now), and keyset
    // pagination forward by ascending id (`id > COALESCE($7, 0)`).
    // `LIMIT $8` is `NULL` when `count` is `None` → unlimited.
    let rows = sqlx::query(
        "SELECT id, \
                name, \
                agent_instance_hierarchy, \
                command, \
                description, \
                created_at, \
                last_ran_at, \
                interval_seconds, \
                plugin_owner, \
                plugin_repository, \
                plugin_version, \
                version \
         FROM schedules \
         WHERE agent_instance_hierarchy = ANY($1) \
             AND ($2 = 0 OR interval_seconds IS NULL) \
             AND ($3 = 0 OR interval_seconds IS NOT NULL) \
             AND ($4 = 0 OR ( \
                 (interval_seconds IS NULL AND last_ran_at IS NULL) \
                 OR \
                 (interval_seconds IS NOT NULL \
                  AND (last_ran_at IS NULL \
                       OR ($5 - last_ran_at) >= interval_seconds)) \
             )) \
             AND ($6 = 0 OR ( \
                 (interval_seconds IS NULL AND last_ran_at IS NOT NULL) \
                 OR \
                 (interval_seconds IS NOT NULL \
                  AND last_ran_at IS NOT NULL \
                  AND ($5 - last_ran_at) < interval_seconds) \
             )) \
             AND id > COALESCE($7, 0) \
         ORDER BY id ASC \
         LIMIT $8",
    )
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
