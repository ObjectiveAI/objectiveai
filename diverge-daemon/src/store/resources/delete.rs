//! Deleting a resource.

use sqlx::PgConnection;

use crate::store::Error;

/// Mark the resource deleted and clear its tags; the row stays, so
/// the same bytes held anew are this one again. Whether a container
/// mounts it is the caller's to have checked, and the bytes are the
/// content store's to remove.
pub async fn delete(conn: &mut PgConnection, id: &str) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.resources SET deleted = true, tags = '{}' WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
