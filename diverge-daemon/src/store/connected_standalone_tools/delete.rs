//! Removing a connected tool.

use sqlx::PgConnection;

use crate::store::{ConnectedToolId, Error};

/// Delete the tool; its attachments go with it. That it is attached
/// nowhere is the caller's to have checked, as the wire refuses the
/// delete of an attached tool.
pub async fn delete(conn: &mut PgConnection, id: ConnectedToolId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.connected_standalone_tools WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
