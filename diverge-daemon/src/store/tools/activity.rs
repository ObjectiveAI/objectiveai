//! Where and when a tool last ran.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::{Error, ToolId};

/// Keep that the tool's container began or ceased running on
/// `provider` — or, for a connected tool, that its connect scope was
/// taken or let go there — at `when`.
pub async fn set_last(conn: &mut PgConnection, id: ToolId, provider: &Identity, when: DateTime<Utc>) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.tools SET last_provider = $2, last_active = $3 WHERE id = $1")
        .bind(id.0)
        .bind(Json(provider))
        .bind(when)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
