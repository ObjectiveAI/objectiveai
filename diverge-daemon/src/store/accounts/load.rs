//! Loading accounts.

use diverge_sdk::daemon::endpoints::accounts::Reference;
use sqlx::{PgConnection, Row as _};

use super::row::{self, GROUP, SELECT};
use super::Account;
use crate::store::{AccountId, Error};

/// Every account, oldest created first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Account>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {GROUP}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::account).collect()
}

/// The account with the id, if any.
pub async fn by_id(conn: &mut PgConnection, id: AccountId) -> Result<Option<Account>, Error> {
    let row = sqlx::query(&format!("{SELECT} WHERE a.id = $1 {GROUP}"))
        .bind(id.0)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::account).transpose()
}

/// The account the reference names, if any — by name, or by its
/// credential's identity. With `lock`, its row is locked for the
/// transaction first, so that two edits of one account take turns;
/// locking and grouping cannot share a statement, so the lock is its
/// own.
pub async fn by_reference(conn: &mut PgConnection, reference: &Reference, lock: bool) -> Result<Option<Account>, Error> {
    let (column, value) = match reference {
        Reference::Name { name } => ("name", name),
        Reference::Identity { identity } => ("identity", identity),
    };
    if lock {
        let locked = sqlx::query(&format!("SELECT id FROM diverge.accounts WHERE {column} = $1 FOR UPDATE"))
            .bind(value)
            .fetch_optional(&mut *conn)
            .await?;
        let Some(locked) = locked else {
            return Ok(None);
        };
        return by_id(conn, AccountId(locked.try_get("id")?)).await;
    }
    let row = sqlx::query(&format!("{SELECT} WHERE a.{column} = $1 {GROUP}"))
        .bind(value)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::account).transpose()
}

/// The account whose credential's key has the hash, if any: how a
/// presented credential is judged.
pub async fn by_key_hash(conn: &mut PgConnection, key_hash: &str) -> Result<Option<Account>, Error> {
    let row = sqlx::query(&format!("{SELECT} WHERE a.key_hash = $1 {GROUP}"))
        .bind(key_hash)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::account).transpose()
}
