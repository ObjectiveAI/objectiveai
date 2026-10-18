//! Reading a dependency tool off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::shared::containers::dependencies::Template;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::{DependencyTool, scope_of};
use crate::store::{AgentId, DependencyToolId, Error};

/// The columns every dependency tool query selects: the row joined to
/// its template's body.
pub(super) const SELECT: &str = "SELECT t.id, t.template, d.template AS declared, t.scope, t.owner_agent, t.owner_template, t.tags, t.created, t.creator \
     FROM diverge.dependency_tools t JOIN diverge.dependencies d ON d.id = t.template";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY t.created, t.id";

/// The tool a row of [`SELECT`] holds.
pub(super) fn tool(row: &PgRow) -> Result<DependencyTool, Error> {
    let scope: String = row.try_get("scope")?;
    let Some(scope) = scope_of(&scope) else {
        return Err(Error::Json(serde::de::Error::custom(format!("`{scope}` is not a dependency tool scope"))));
    };
    let owner_agent: Option<i64> = row.try_get("owner_agent")?;
    let Json(declared): Json<Template> = row.try_get("declared")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(DependencyTool {
        id: DependencyToolId(row.try_get("id")?),
        template: row.try_get("template")?,
        declared,
        scope,
        owner_agent: owner_agent.map(AgentId),
        owner_template: row.try_get("owner_template")?,
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
