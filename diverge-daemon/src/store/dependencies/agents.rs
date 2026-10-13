//! What each agent depends on: the templates its last registration
//! declared.
//!
//! Every registration RESETS the set: what the agent declared then is
//! what it depends on, a template declared before and not again is
//! its dependency no more, and a start whose program declared nothing
//! leaves it depending on nothing. A template kept from one
//! registration to the next keeps its place.

use sqlx::{PgConnection, Row as _};

use crate::store::{AgentId, Error};

/// One dependence: the agent, the template, and its place in the
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependence {
    /// The agent.
    pub agent: AgentId,
    /// The dependency template, by id.
    pub template: String,
    /// The order of first declaring, across every agent and template.
    pub seq: i64,
}

/// Every dependence, in order: what a listing folds in, loaded once
/// for the whole list.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Dependence>, Error> {
    let rows = sqlx::query("SELECT agent, template, seq FROM diverge.agent_dependencies ORDER BY seq")
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(dependence).collect()
}

/// The agents that depend on the template now, in order.
pub async fn of_template(conn: &mut PgConnection, template: &str) -> Result<Vec<AgentId>, Error> {
    let rows = sqlx::query("SELECT agent, template, seq FROM diverge.agent_dependencies WHERE template = $1 ORDER BY seq")
        .bind(template)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| dependence(row).map(|dependence| dependence.agent)).collect()
}

/// Make the agent's dependencies exactly `templates`: every other
/// taken off, every one not held already put on after the rest, and
/// one held already left in its place.
pub async fn replace(conn: &mut PgConnection, agent: AgentId, templates: &[String]) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.agent_dependencies WHERE agent = $1 AND NOT (template = ANY($2))")
        .bind(agent.0)
        .bind(templates)
        .execute(&mut *conn)
        .await?;
    for template in templates {
        sqlx::query("INSERT INTO diverge.agent_dependencies (agent, template) VALUES ($1, $2) ON CONFLICT (agent, template) DO NOTHING")
            .bind(agent.0)
            .bind(template)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// The template is deleted: no agent depends on it any more, until a
/// registration declares it anew.
pub async fn remove_template(conn: &mut PgConnection, template: &str) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.agent_dependencies WHERE template = $1")
        .bind(template)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// The dependence a row holds.
fn dependence(row: &sqlx::postgres::PgRow) -> Result<Dependence, Error> {
    Ok(Dependence {
        agent: AgentId(row.try_get("agent")?),
        template: row.try_get("template")?,
        seq: row.try_get("seq")?,
    })
}
