//! Loading tool templates.

use sqlx::PgConnection;

use super::Record;
use super::row::{self, ORDER, SELECT};
use crate::store::Error;

/// Every live tool template, oldest first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Record>, Error> {
    let rows = sqlx::query(&format!("{SELECT} WHERE NOT deleted {ORDER}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::record).collect()
}

/// The live template with the id, if any. With `lock`, its row is
/// locked for the transaction.
pub async fn by_id(conn: &mut PgConnection, id: &str, lock: bool) -> Result<Option<Record>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE id = $1 AND NOT deleted{locking}"))
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::record).transpose()
}
