//! When the daemon's connection to a provider last changed.

use sqlx::PgConnection;

use crate::store::{Error, OutgoingId};

/// Record that the connection to the provider opened or closed now.
/// A row that is gone — the provider forgotten while its dial was
/// ending — is nothing to record.
pub async fn set_last_connected(conn: &mut PgConnection, id: OutgoingId) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers_outgoing SET last_connected = now() WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
