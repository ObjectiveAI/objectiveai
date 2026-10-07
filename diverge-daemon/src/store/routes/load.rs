//! Loading routes.

use diverge_sdk::daemon::endpoints::tools::routes::Path;
use sqlx::PgConnection;

use super::Route;
use super::row::{self, ORDER, SELECT};
use crate::store::{Error, ToolId};

/// Every route, oldest first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Route>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::route).collect()
}

/// The route at the position, if any. With `lock`, its row is locked
/// for the transaction.
pub async fn by_path(conn: &mut PgConnection, path: &Path, lock: bool) -> Result<Option<Route>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE agent = $1 AND templates = $2{locking}"))
        .bind(&path.agent)
        .bind(&path.templates)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::route).transpose()
}

/// The positions routed to the tool, oldest first.
pub async fn of_tool(conn: &mut PgConnection, tool: ToolId) -> Result<Vec<Path>, Error> {
    let rows = sqlx::query(&format!("{SELECT} WHERE tool = $1 {ORDER}"))
        .bind(tool.0)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| row::route(row).map(|route| route.path())).collect()
}
