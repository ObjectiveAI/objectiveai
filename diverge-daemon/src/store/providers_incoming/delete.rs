//! Taking an incoming credential out.

use sqlx::PgConnection;

use crate::store::{Error, IncomingId};

/// Delete the credential; its key admits nothing from then on.
/// Whether a provider is connected through it is the caller's to
/// have checked.
pub async fn delete(conn: &mut PgConnection, id: IncomingId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.providers_incoming WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
