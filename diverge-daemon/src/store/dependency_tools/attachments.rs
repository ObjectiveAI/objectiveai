//! Agents attached to dependency tools, in the order they attached,
//! each with when its registration last declared the template.
//!
//! An agent is attached to a tool by a registration that declares the
//! tool's template, which stamps `declared`, and detached by one that
//! no longer does; what holds a service up is that some agent is
//! attached to it. An instance tool has at most one attachment, its
//! owner's.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, Row as _};

use crate::store::{AgentId, DependencyToolId, Error};

/// One attachment: the tool, the agent, its place in the order, and
/// when the agent last declared the template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attachment {
    /// The tool.
    pub tool: DependencyToolId,
    /// The agent attached to it.
    pub agent: AgentId,
    /// The order of first attaching, across every tool and agent.
    pub seq: i64,
    /// When the agent's registration last declared the template.
    pub declared: DateTime<Utc>,
}

/// Every attachment, in attach order: what a listing folds in, loaded
/// once for the whole list.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Attachment>, Error> {
    let rows = sqlx::query("SELECT tool, agent, seq, declared FROM diverge.dependency_attachments ORDER BY seq")
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(attachment).collect()
}

/// The attachments to the tool, in attach order.
pub async fn attached(conn: &mut PgConnection, tool: DependencyToolId) -> Result<Vec<Attachment>, Error> {
    let rows = sqlx::query("SELECT tool, agent, seq, declared FROM diverge.dependency_attachments WHERE tool = $1 ORDER BY seq")
        .bind(tool.0)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(attachment).collect()
}

/// Attach the agent to the tool, as of now: one attached already
/// keeps its place and has its `declared` moved to now.
pub async fn attach(conn: &mut PgConnection, tool: DependencyToolId, agent: AgentId) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO diverge.dependency_attachments (tool, agent) VALUES ($1, $2) \
         ON CONFLICT (tool, agent) DO UPDATE SET declared = now()",
    )
    .bind(tool.0)
    .bind(agent.0)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Detach the agent from every tool but those in `keep`: what a
/// registration does for the tools it no longer declares. The tools
/// detached from, for their holds to be told.
pub async fn detach_except(conn: &mut PgConnection, agent: AgentId, keep: &[DependencyToolId]) -> Result<Vec<DependencyToolId>, Error> {
    let kept: Vec<i64> = keep.iter().map(|tool| tool.0).collect();
    let rows = sqlx::query("DELETE FROM diverge.dependency_attachments WHERE agent = $1 AND NOT (tool = ANY($2)) RETURNING tool")
        .bind(agent.0)
        .bind(&kept)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| Ok(DependencyToolId(row.try_get("tool")?))).collect()
}

/// The attachment a row holds.
fn attachment(row: &sqlx::postgres::PgRow) -> Result<Attachment, Error> {
    Ok(Attachment {
        tool: DependencyToolId(row.try_get("tool")?),
        agent: AgentId(row.try_get("agent")?),
        seq: row.try_get("seq")?,
        declared: row.try_get("declared")?,
    })
}
