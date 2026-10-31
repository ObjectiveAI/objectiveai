//! When an agent last changed: deployed, and active.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::agent_log::Identity;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::{AgentId, Error};

/// Keep that its container came up or went down on `provider` at
/// `when`: what a list reports as its provider and its `last_deployed`.
pub async fn set_deployed(conn: &mut PgConnection, id: AgentId, provider: &Identity, when: DateTime<Utc>) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.agents SET last_provider = $2, last_deployed = $3 WHERE id = $1")
        .bind(id.0)
        .bind(Json(provider))
        .bind(when)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Keep that it began or ceased working at `when`: what a list reports
/// as its `last_active`.
pub async fn set_active(conn: &mut PgConnection, id: AgentId, when: DateTime<Utc>) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.agents SET last_active = $2 WHERE id = $1")
        .bind(id.0)
        .bind(when)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
