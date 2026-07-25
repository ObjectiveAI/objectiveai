//! Neutral row types for the `tasks` DB layer — free of the SDK
//! `tasks` command types; the daemon handlers map these to the wire
//! response types.

use objectiveai_sdk::identity::Identity;
use sqlx::Row as _;

/// Reassemble the creator identity from its per-field columns (no
/// JSON blob). The `task` flag is NOT stored — the scheduler stamps
/// it fresh on every fire ([`crate::context::ScopedContext::with_task`]).
pub(super) fn identity_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<Identity, sqlx::Error> {
    Ok(Identity {
        agent_instance_hierarchy: row.try_get("agent_instance_hierarchy")?,
        agent_id: row.try_get("agent_id")?,
        agent_full_id: row.try_get("agent_full_id")?,
        agent_remote: row.try_get("agent_remote")?,
        response_id: row.try_get("response_id")?,
        response_ids: row.try_get("response_ids")?,
        plugin_owner: row.try_get("plugin_owner")?,
        plugin_name: row.try_get("plugin_name")?,
        plugin_version: row.try_get("plugin_version")?,
        task: false,
    })
}

/// The identity columns, in select order — shared by the claim and
/// list queries so [`identity_from_row`] can read either.
pub(super) const IDENTITY_COLUMNS: &str =
    "plugin_owner, plugin_name, plugin_version, agent_instance_hierarchy, \
     agent_id, agent_full_id, agent_remote, response_id, response_ids";

/// One stored task, as `tasks list` reads it.
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: String,
    /// The stored command-request JSON, verbatim.
    pub command: serde_json::Value,
    pub identity: Identity,
    pub delay_secs: i64,
    pub repeat: bool,
    pub repeat_count: Option<i64>,
    pub run_count: i64,
    pub error_count: i64,
    /// `"success"` / `"error"` / `None` (never ran).
    pub last_result: Option<String>,
    /// `true` = will never fire again (stays listed until deleted).
    pub complete: bool,
    pub created_at: i64,
    pub next_run_at: Option<i64>,
}

/// One task the scheduler just CLAIMED (`scheduled` → `running`) —
/// everything a fire needs. The completion targets it by its sole
/// identity: `id` within the namespace of `identity`'s plugin
/// trio.
#[derive(Debug, Clone)]
pub struct ClaimedTask {
    pub id: String,
    pub command: serde_json::Value,
    pub identity: Identity,
}
