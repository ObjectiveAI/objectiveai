//! Removing an account.

use sqlx::PgConnection;

use crate::store::{AccountId, Error};

/// Delete the account; its holdings of roles go with it, and so does
/// every event it is an end of — taken here, since an event holds an
/// account end by its identity, which no reference cascades. Whether
/// anything holds the account — a client connected as it, a container
/// running under it — is the caller's to have checked, since neither
/// is a row here.
pub async fn delete(conn: &mut PgConnection, id: AccountId) -> Result<(), Error> {
    sqlx::query(
        "DELETE FROM diverge.events e USING diverge.accounts a WHERE a.id = $1 \
         AND ((e.sender_kind = 'account' AND e.sender_issuer = a.issuer AND e.sender_subject = a.subject) \
         OR (e.receiver_kind = 'account' AND e.receiver_issuer = a.issuer AND e.receiver_subject = a.subject))",
    )
    .bind(id.0)
    .execute(&mut *conn)
    .await?;
    sqlx::query("DELETE FROM diverge.accounts WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
