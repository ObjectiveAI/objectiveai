//! Deleting a dependency template.

use sqlx::PgConnection;

use crate::store::Error;

/// Mark the template deleted and clear its tags; the row stays, so a
/// registration declaring the same template later makes this one
/// again. Whether a tool runs from it is the caller's to have checked.
pub async fn delete(conn: &mut PgConnection, id: &str) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.dependencies SET deleted = true, tags = '{}' WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
