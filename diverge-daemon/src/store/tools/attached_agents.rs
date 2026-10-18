//! The agents that have a tool: attached to it directly, or made from
//! a template it is attached to.

use sqlx::{PgConnection, Row as _};

use crate::store::{AgentId, Error, ToolId};

/// Every agent that has the tool now: those it is attached to
/// directly, and every agent of every template it is attached to,
/// each once, in no order. What a service's hold, a grant's facts and
/// a live attach or detach are judged by.
pub async fn attached_agents(conn: &mut PgConnection, tool: ToolId) -> Result<Vec<AgentId>, Error> {
    let rows = sqlx::query(
        "SELECT agent AS id FROM diverge.attachments WHERE tool = $1
         UNION
         SELECT id FROM diverge.agents WHERE template IN (SELECT template FROM diverge.template_attachments WHERE tool = $1)",
    )
    .bind(tool.0)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.iter().map(|row| AgentId(row.get("id"))).collect())
}

/// Every agent made from the template, oldest first: who a template
/// attachment reaches.
pub async fn agents_of_template(conn: &mut PgConnection, template: &str) -> Result<Vec<AgentId>, Error> {
    let rows = sqlx::query("SELECT id FROM diverge.agents WHERE template = $1 ORDER BY id")
        .bind(template)
        .fetch_all(&mut *conn)
        .await?;
    Ok(rows.iter().map(|row| AgentId(row.get("id"))).collect())
}
