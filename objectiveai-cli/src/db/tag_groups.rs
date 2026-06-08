//! `tag_groups` table — explicit grouping container that lets
//! many `tags` rows share one resolved `AgentSpec` + parent
//! lineage.
//!
//! Inserts happen inline in `db::tags::apply`'s
//! `ResolvedApplyTarget::Agent` arm (kept there so the
//! `tags`-side write and the `tag_groups`-side write live in the
//! same transaction). This module only exposes the read path:
//! [`fetch`] returns the [`TagGroup`] for a given id, used by
//! `agents spawn` when it resolves a `--agent-tag`
//! invocation into a runnable `AgentSpec` + parent.

use objectiveai_sdk::cli::command::agents::spawn::AgentSpec;
use sqlx::Row as _;

use super::{Error, Pool};

#[derive(Debug, Clone)]
pub struct TagGroup {
    pub id: i64,
    pub agent_spec: AgentSpec,
    pub parent_agent_instance_hierarchy: String,
}

/// Look up a tag_group row by id. Returns `Ok(None)` when no row
/// matches (typically only happens if the caller raced a
/// `DELETE` from the parent `tags` cascade).
pub async fn fetch(pool: &Pool, id: i64) -> Result<Option<TagGroup>, Error> {
    let row = sqlx::query(
        "SELECT id, agent_spec, parent_agent_instance_hierarchy \
         FROM tag_groups \
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&**pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let id: i64 = row.try_get(0)?;
    let spec_value: serde_json::Value = row.try_get(1)?;
    let agent_spec: AgentSpec = serde_json::from_value(spec_value)?;
    let parent: String = row.try_get(2)?;
    Ok(Some(TagGroup {
        id,
        agent_spec,
        parent_agent_instance_hierarchy: parent,
    }))
}
