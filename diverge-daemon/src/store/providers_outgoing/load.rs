//! Loading outgoing providers.

use sqlx::PgConnection;

use super::Outgoing;
use super::row::{self, ORDER, SELECT};
use crate::store::Error;

/// Every outgoing provider, oldest added first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Outgoing>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::outgoing).collect()
}

/// The provider at the address, if any. With `lock`, its row is
/// locked for the transaction.
pub async fn by_address(conn: &mut PgConnection, address: &str, lock: bool) -> Result<Option<Outgoing>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE address = $1{locking}"))
        .bind(address)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::outgoing).transpose()
}
