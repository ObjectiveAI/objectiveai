//! The agents that have a connected tool: attached to it directly, or
//! made from a template it is attached to.

use sqlx::{PgConnection, Row as _};

use crate::store::{AgentId, ConnectedToolId, Error};

/// Every agent that has the tool now: those it is attached to
/// directly, and every agent of every template it is attached to,
/// each once, in no order. What a live detach is judged by.
pub async fn attached_agents(conn: &mut PgConnection, tool: ConnectedToolId) -> Result<Vec<AgentId>, Error> {
    let rows = sqlx::query(
        "SELECT agent AS id FROM diverge.connected_attachments WHERE tool = $1
         UNION
         SELECT id FROM diverge.agents WHERE template IN (SELECT template FROM diverge.connected_template_attachments WHERE tool = $1)",
    )
    .bind(tool.0)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.iter().map(|row| AgentId(row.get("id"))).collect())
}
