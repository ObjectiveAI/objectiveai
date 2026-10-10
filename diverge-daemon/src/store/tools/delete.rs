//! Removing a tool.

use sqlx::PgConnection;

use crate::store::{Error, ToolId};

/// Delete the tool; its attachments and its admissions go with it. That it is attached nowhere is the
/// caller's to have checked, as the wire refuses the delete of an
/// attached tool.
pub async fn delete(conn: &mut PgConnection, id: ToolId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.tools WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
