//! Loading incoming credentials.

use sqlx::PgConnection;

use super::Incoming;
use super::row::{self, ORDER, SELECT};
use crate::store::Error;

/// Every credential, oldest added first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Incoming>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::incoming).collect()
}

/// The credential naming the identity, if any. With `lock`, its row
/// is locked for the transaction.
pub async fn by_identity(conn: &mut PgConnection, identity: &str, lock: bool) -> Result<Option<Incoming>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE identity = $1{locking}"))
        .bind(identity)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::incoming).transpose()
}

/// The credential whose key has the hash, if any: how a presented
/// credential is judged.
pub async fn by_key_hash(conn: &mut PgConnection, key_hash: &str) -> Result<Option<Incoming>, Error> {
    let row = sqlx::query(&format!("{SELECT} WHERE key_hash = $1"))
        .bind(key_hash)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::incoming).transpose()
}
