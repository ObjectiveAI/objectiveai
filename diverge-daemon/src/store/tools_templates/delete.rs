//! Deleting a tool template.

use sqlx::PgConnection;

use crate::store::Error;

/// Mark the template deleted and clear its tags; the row stays, so a
/// create of the same template later is this one again. Whether an
/// tool was made from it is the caller's to have checked.
pub async fn delete(conn: &mut PgConnection, id: &str) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.tools_templates SET deleted = true, tags = '{}' WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
