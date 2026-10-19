//! The accounts on a daemon record: added, rotated, forgotten, loaded.

use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use sqlx::PgConnection;
use sqlx::types::Json;

use super::Account;
use super::row::{self, ORDER_ACCOUNT, SELECT_ACCOUNT};
use crate::store::{DaemonId, Error};

/// An add's outcome: the account made, or the name taken on that
/// daemon already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Added {
    /// Made.
    Added,
    /// The daemon has an account under that name already; nothing was
    /// made.
    Exists,
}

/// The daemon's accounts, oldest first.
pub async fn of_daemon(conn: &mut PgConnection, daemon: DaemonId) -> Result<Vec<Account>, Error> {
    let rows = sqlx::query(&format!("{SELECT_ACCOUNT} WHERE daemon = $1 {ORDER_ACCOUNT}"))
        .bind(daemon.0)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| row::account(row).map(|(_, account)| account)).collect()
}

/// Add an account to the daemon.
pub async fn add(conn: &mut PgConnection, daemon: DaemonId, name: &str, mode: &Mode) -> Result<Added, Error> {
    let inserted = sqlx::query("INSERT INTO diverge.providers_daemons_accounts (daemon, name, mode) VALUES ($1, $2, $3)")
        .bind(daemon.0)
        .bind(name)
        .bind(Json(mode))
        .execute(&mut *conn)
        .await;
    match inserted {
        Ok(_) => Ok(Added::Added),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Added::Exists),
        Err(error) => Err(error.into()),
    }
}

/// Replace the account's mode. Whether there was one to replace.
pub async fn set_mode(conn: &mut PgConnection, daemon: DaemonId, name: &str, mode: &Mode) -> Result<bool, Error> {
    let done = sqlx::query("UPDATE diverge.providers_daemons_accounts SET mode = $3 WHERE daemon = $1 AND name = $2")
        .bind(daemon.0)
        .bind(name)
        .bind(Json(mode))
        .execute(&mut *conn)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Forget the account. Whether there was one to forget.
pub async fn delete(conn: &mut PgConnection, daemon: DaemonId, name: &str) -> Result<bool, Error> {
    let done = sqlx::query("DELETE FROM diverge.providers_daemons_accounts WHERE daemon = $1 AND name = $2")
        .bind(daemon.0)
        .bind(name)
        .execute(&mut *conn)
        .await?;
    Ok(done.rows_affected() == 1)
}
