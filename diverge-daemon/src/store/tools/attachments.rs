//! Tools attached to agents, in the order they were attached.

use sqlx::{PgConnection, Row as _};

use crate::store::{AgentId, Error, ToolId};

/// One attachment: the tool, the agent, and its place in the order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attachment {
    /// The tool.
    pub tool: ToolId,
    /// The agent it is attached to.
    pub agent: AgentId,
    /// The order of attaching, across every agent and tool.
    pub seq: i64,
}

/// Every attachment, in attach order: what a listing of either family
/// folds in, loaded once for the whole list.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Attachment>, Error> {
    let rows = sqlx::query("SELECT tool, agent, seq FROM diverge.attachments ORDER BY seq")
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(attachment).collect()
}

/// The agents the tool is attached to, in attach order.
pub async fn of_tool(conn: &mut PgConnection, tool: ToolId) -> Result<Vec<AgentId>, Error> {
    let rows = sqlx::query("SELECT tool, agent, seq FROM diverge.attachments WHERE tool = $1 ORDER BY seq")
        .bind(tool.0)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| attachment(row).map(|attachment| attachment.agent)).collect()
}

/// The tools attached to the agent, in attach order.
pub async fn of_agent(conn: &mut PgConnection, agent: AgentId) -> Result<Vec<ToolId>, Error> {
    let rows = sqlx::query("SELECT tool, agent, seq FROM diverge.attachments WHERE agent = $1 ORDER BY seq")
        .bind(agent.0)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| attachment(row).map(|attachment| attachment.tool)).collect()
}

/// Attach the tool to the agent. One already attached stays as it
/// was, in its place.
pub async fn attach(conn: &mut PgConnection, tool: ToolId, agent: AgentId) -> Result<(), Error> {
    sqlx::query("INSERT INTO diverge.attachments (tool, agent) VALUES ($1, $2) ON CONFLICT (tool, agent) DO NOTHING")
        .bind(tool.0)
        .bind(agent.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Detach the tool from the agent. One not attached is nothing to
/// take back.
pub async fn detach(conn: &mut PgConnection, tool: ToolId, agent: AgentId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.attachments WHERE tool = $1 AND agent = $2")
        .bind(tool.0)
        .bind(agent.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// The attachment a row holds.
fn attachment(row: &sqlx::postgres::PgRow) -> Result<Attachment, Error> {
    Ok(Attachment {
        tool: ToolId(row.try_get("tool")?),
        agent: AgentId(row.try_get("agent")?),
        seq: row.try_get("seq")?,
    })
}
