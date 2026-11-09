//! Loading tools.

use diverge_sdk::daemon::reference;
use sqlx::{PgConnection, Row as _};

use super::Tool;
use super::row::{self, ORDER, SELECT};
use crate::store::{Error, ToolId};

/// Every tool, oldest first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Tool>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::tool).collect()
}

/// The tool with the id, if any. With `lock`, its row is locked for
/// the transaction.
pub async fn by_id(conn: &mut PgConnection, id: ToolId, lock: bool) -> Result<Option<Tool>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE id = $1{locking}"))
        .bind(id.0)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::tool).transpose()
}

/// The tool the reference names, if any — by name, or by template and
/// index. With `lock`, its row is locked for the transaction, so that
/// two edits of one tool take turns. A connected tool is none of
/// these: see
/// [`connected_standalone_tools`](crate::store::connected_standalone_tools).
pub async fn by_reference(conn: &mut PgConnection, reference: &reference::Tool, lock: bool) -> Result<Option<Tool>, Error> {
    let found = match reference {
        reference::Tool::Name { name } => {
            sqlx::query("SELECT id FROM diverge.tools WHERE name = $1")
                .bind(name)
                .fetch_optional(&mut *conn)
                .await?
        }
        reference::Tool::TemplateIndex { template, index } => {
            sqlx::query("SELECT id FROM diverge.tools WHERE template = $1 AND index = $2")
                .bind(template)
                .bind(i64::try_from(*index).unwrap_or(i64::MAX))
                .fetch_optional(&mut *conn)
                .await?
        }
    };
    let Some(found) = found else {
        return Ok(None);
    };
    by_id(conn, ToolId(found.try_get("id")?), lock).await
}
