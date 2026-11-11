//! When an account last changed: connected.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::store::{AccountId, Error};

/// Keep that a client's first connection as the account began, or its
/// last one ended, at `when`: what a list reports as its
/// `last_connected`.
pub async fn set_connected(conn: &mut PgConnection, id: AccountId, when: DateTime<Utc>) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.accounts SET last_connected = $2 WHERE id = $1")
        .bind(id.0)
        .bind(when)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
