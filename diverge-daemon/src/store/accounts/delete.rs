//! Removing an account.

use sqlx::PgConnection;

use crate::store::{AccountId, Error};

/// Delete the account; its holdings of roles go with it. Whether
/// anything holds the account — a client connected as it, a container
/// running under it — is the caller's to have checked, since neither
/// is a row here.
pub async fn delete(conn: &mut PgConnection, id: AccountId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.accounts WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
