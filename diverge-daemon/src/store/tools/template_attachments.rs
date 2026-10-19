//! Tools attached to agent templates — to every agent of each — in
//! the order they were attached.

use sqlx::{PgConnection, Row as _};

use crate::store::{Error, ToolId};

/// One attachment: the tool, the template, and its place in the
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateAttachment {
    /// The tool.
    pub tool: ToolId,
    /// The agent template it is attached to, by id.
    pub template: String,
    /// The order of attaching, across every template and tool.
    pub seq: i64,
}

/// Every attachment, in attach order: what a listing folds in,
/// loaded once for the whole list.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<TemplateAttachment>, Error> {
    let rows = sqlx::query("SELECT tool, template, seq FROM diverge.template_attachments ORDER BY seq")
        .fetch_all(&mut *conn)
        .await?;
    Ok(rows
        .iter()
        .map(|row| TemplateAttachment {
            tool: ToolId(row.get("tool")),
            template: row.get("template"),
            seq: row.get("seq"),
        })
        .collect())
}

/// The templates the tool is attached to, by id, in attach order.
pub async fn of_tool(conn: &mut PgConnection, tool: ToolId) -> Result<Vec<String>, Error> {
    let rows = sqlx::query("SELECT template FROM diverge.template_attachments WHERE tool = $1 ORDER BY seq")
        .bind(tool.0)
        .fetch_all(&mut *conn)
        .await?;
    Ok(rows.iter().map(|row| row.get("template")).collect())
}

/// The tools attached to the template, in attach order.
pub async fn of_template(conn: &mut PgConnection, template: &str) -> Result<Vec<ToolId>, Error> {
    let rows = sqlx::query("SELECT tool FROM diverge.template_attachments WHERE template = $1 ORDER BY seq")
        .bind(template)
        .fetch_all(&mut *conn)
        .await?;
    Ok(rows.iter().map(|row| ToolId(row.get("tool"))).collect())
}

/// Attach the tool to the template. One already attached stays as it
/// was, in its place.
pub async fn attach(conn: &mut PgConnection, tool: ToolId, template: &str) -> Result<(), Error> {
    sqlx::query("INSERT INTO diverge.template_attachments (tool, template) VALUES ($1, $2) ON CONFLICT (tool, template) DO NOTHING")
        .bind(tool.0)
        .bind(template)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Detach the tool from the template. One not attached is nothing to
/// take back.
pub async fn detach(conn: &mut PgConnection, tool: ToolId, template: &str) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.template_attachments WHERE tool = $1 AND template = $2")
        .bind(tool.0)
        .bind(template)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Every attachment to the template taken off, for its delete: the
/// tools that were attached, in attach order, each to be told its
/// hold changed.
pub async fn clear_template(conn: &mut PgConnection, template: &str) -> Result<Vec<ToolId>, Error> {
    let tools = of_template(conn, template).await?;
    sqlx::query("DELETE FROM diverge.template_attachments WHERE template = $1")
        .bind(template)
        .execute(&mut *conn)
        .await?;
    Ok(tools)
}
