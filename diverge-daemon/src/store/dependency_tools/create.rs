//! Making a dependency tool, or finding the one its scope key names.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::shared::containers::dependencies::Scope;
use sqlx::PgConnection;
use sqlx::types::Json;

use super::{DependencyTool, by_scope, scope_name};
use crate::store::{AgentId, Error};

/// What a registration declares, checked by the caller: the template
/// by id, whose body is on record, its scope — `instance` for a
/// transient template — and the agent registering, whose id and
/// template the scope key is made of.
#[derive(Debug, Clone)]
pub struct Declared<'a> {
    /// The template, by id.
    pub template: &'a str,
    /// How far the tool is shared.
    pub scope: Scope,
    /// The agent registering.
    pub agent: AgentId,
    /// The agent's template, by id.
    pub agent_template: &'a str,
    /// The agent, as the creator of a tool made now.
    pub creator: &'a Creator,
}

/// The tool under the declaration's scope key: the one there is, or
/// one made now. Two registrations racing to make one both come to
/// the one row: the insert yields to a unique violation on the scope
/// key, and the row is read again.
pub async fn find_or_create(conn: &mut PgConnection, declared: &Declared<'_>) -> Result<DependencyTool, Error> {
    let (owner_agent, owner_template) = match declared.scope {
        Scope::Instance => (Some(declared.agent), None),
        Scope::Template => (None, Some(declared.agent_template)),
        Scope::Global => (None, None),
    };
    if let Some(found) = by_scope(conn, declared.template, declared.scope, owner_agent, owner_template, false).await? {
        return Ok(found);
    }
    sqlx::query(
        "INSERT INTO diverge.dependency_tools (template, scope, owner_agent, owner_template, creator) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
    )
    .bind(declared.template)
    .bind(scope_name(declared.scope))
    .bind(owner_agent.map(|agent| agent.0))
    .bind(owner_template)
    .bind(Json(declared.creator))
    .execute(&mut *conn)
    .await?;
    by_scope(conn, declared.template, declared.scope, owner_agent, owner_template, false)
        .await?
        .ok_or_else(|| Error::Json(serde::de::Error::custom("the dependency tool made is not there")))
}
