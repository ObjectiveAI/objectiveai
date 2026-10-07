//! Forgetting an outgoing provider.

use sqlx::PgConnection;

use crate::store::{Error, OutgoingId};

/// Delete the provider. Whether a container is pinned to it, or
/// mounts a volume of its, is the caller's to have checked.
pub async fn delete(conn: &mut PgConnection, id: OutgoingId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.providers_outgoing WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
