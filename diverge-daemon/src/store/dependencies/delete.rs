//! Removing a dependency template's body.

use sqlx::PgConnection;

use crate::store::Error;

/// Delete the body. That no tool of the template is left is the
/// caller's to have seen to; one already gone is nothing to do.
pub async fn delete(conn: &mut PgConnection, id: &str) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.dependencies WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
