//! Loading vault keys.

use sqlx::PgConnection;

use super::Entry;
use super::row::{self, ORDER, SELECT};
use crate::store::Error;

/// Every key, oldest first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Entry>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    row::entries(conn, &rows).await
}

/// The key, if the vault holds it. With `lock`, its row is locked
/// for the transaction.
pub async fn by_key(conn: &mut PgConnection, key: &str, lock: bool) -> Result<Option<Entry>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let rows = sqlx::query(&format!("{SELECT} WHERE key = $1{locking}"))
        .bind(key)
        .fetch_all(&mut *conn)
        .await?;
    Ok(row::entries(conn, &rows).await?.pop())
}
