//! Removing an agent.

use sqlx::PgConnection;

use crate::store::{AgentId, Error};

/// Delete the agent; its attachments go with it. Whether a loop runs
/// in it is the caller's to have checked, and its container and its
/// log are the caller's to end and remove: neither is a row here.
pub async fn delete(conn: &mut PgConnection, id: AgentId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.agents WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
