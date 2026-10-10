//! Loading daemon records.

use sqlx::PgConnection;

use super::Record;
use super::row::{self, ORDER, SELECT};
use crate::store::Error;

/// Every daemon record, oldest added first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Record>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::record).collect()
}

/// The record under the name, if any. With `lock`, its row is locked
/// for the transaction.
pub async fn by_name(conn: &mut PgConnection, name: &str, lock: bool) -> Result<Option<Record>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE name = $1{locking}"))
        .bind(name)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::record).transpose()
}
