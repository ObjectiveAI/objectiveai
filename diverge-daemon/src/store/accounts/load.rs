//! Loading accounts.

use diverge_sdk::shared::Identity;
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

/// The account `identity` is, if any. With `lock`, its row is locked for the
/// transaction first, so that two edits of one account take turns;
/// locking and grouping cannot share a statement, so the lock is its
/// own.
pub async fn by_identity(conn: &mut PgConnection, identity: &Identity, lock: bool) -> Result<Option<Account>, Error> {
    if lock {
        let locked = sqlx::query("SELECT id FROM diverge.accounts WHERE issuer = $1 AND subject = $2 FOR UPDATE")
            .bind(&identity.issuer)
            .bind(&identity.subject)
            .fetch_optional(&mut *conn)
            .await?;
        let Some(locked) = locked else {
            return Ok(None);
        };
        return by_id(conn, AccountId(locked.try_get("id")?)).await;
    }
    let row = sqlx::query(&format!("{SELECT} WHERE a.issuer = $1 AND a.subject = $2 {GROUP}"))
        .bind(&identity.issuer)
        .bind(&identity.subject)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::account).transpose()
}
