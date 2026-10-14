//! Loading daemon records, each with its accounts.

use std::collections::HashMap;

use sqlx::PgConnection;

use super::row::{self, ORDER, ORDER_ACCOUNT, SELECT, SELECT_ACCOUNT};
use super::{Account, Record};
use crate::store::{DaemonId, Error};

/// Every record, oldest first, each with its accounts.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Record>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    let mut records: Vec<Record> = rows.iter().map(row::record).collect::<Result<_, _>>()?;
    let rows = sqlx::query(&format!("{SELECT_ACCOUNT} {ORDER_ACCOUNT}")).fetch_all(&mut *conn).await?;
    let mut accounts: HashMap<DaemonId, Vec<Account>> = HashMap::new();
    for row in &rows {
        let (daemon, account) = row::account(row)?;
        accounts.entry(daemon).or_default().push(account);
    }
    for record in &mut records {
        record.accounts = accounts.remove(&record.id).unwrap_or_default();
    }
    Ok(records)
}

/// The record of the name, with its accounts, locked for the
/// transaction when `lock`.
pub async fn by_name(conn: &mut PgConnection, name: &str, lock: bool) -> Result<Option<Record>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE name = $1{locking}"))
        .bind(name)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(mut record) = row.as_ref().map(row::record).transpose()? else {
        return Ok(None);
    };
    record.accounts = super::accounts::of_daemon(conn, record.id).await?;
    Ok(Some(record))
}
