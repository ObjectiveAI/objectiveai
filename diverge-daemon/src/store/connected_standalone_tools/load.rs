//! Loading connected tools.

use diverge_sdk::daemon::reference;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use super::ConnectedTool;
use super::row::{self, ORDER, SELECT};
use crate::store::{ConnectedToolId, Error, daemons};

/// Every connected tool, oldest first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<ConnectedTool>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::tool).collect()
}

/// The tool with the id, if any. With `lock`, its row is locked for
/// the transaction.
pub async fn by_id(conn: &mut PgConnection, id: ConnectedToolId, lock: bool) -> Result<Option<ConnectedTool>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE id = $1{locking}"))
        .bind(id.0)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::tool).transpose()
}

/// The tool the reference names, if any — by name, or by the daemon,
/// the tool there and its index. With `lock`, its row is locked for the
/// transaction, so that two edits of one tool take turns. A name a
/// standalone tool holds finds nothing here: the namespace is one, see
/// [`tool_names`](crate::store::tool_names).
pub async fn by_reference(conn: &mut PgConnection, reference: &reference::ConnectedStandaloneTool, lock: bool) -> Result<Option<ConnectedTool>, Error> {
    let found = match reference {
        reference::ConnectedStandaloneTool::Name { name } => {
            sqlx::query("SELECT id FROM diverge.connected_standalone_tools WHERE name = $1")
                .bind(name)
                .fetch_optional(&mut *conn)
                .await?
        }
        reference::ConnectedStandaloneTool::DaemonToolIndex { daemon, tool, index } => {
            sqlx::query("SELECT id FROM diverge.connected_standalone_tools WHERE daemon = $1 AND tool = $2 AND index = $3")
                .bind(daemons::to_column(*daemon))
                .bind(Json(tool.as_ref()))
                .bind(i64::try_from(*index).unwrap_or(i64::MAX))
                .fetch_optional(&mut *conn)
                .await?
        }
    };
    let Some(found) = found else {
        return Ok(None);
    };
    by_id(conn, ConnectedToolId(found.try_get("id")?), lock).await
}
