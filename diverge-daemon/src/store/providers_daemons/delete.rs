//! Forgetting a daemon record.

use sqlx::PgConnection;

use crate::store::{DaemonId, Error};

/// Delete the record. Whether a connected tool names it is the
/// caller's to have checked.
pub async fn delete(conn: &mut PgConnection, id: DaemonId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.providers_daemons WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
