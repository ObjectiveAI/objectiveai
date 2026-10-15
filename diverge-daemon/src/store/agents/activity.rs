//! Where and when an agent last ran.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::{AgentId, Error};

/// Keep that the agent began or ceased running on `provider` at
/// `when`: what a list reports as its provider and its `last_active`,
/// the `created` of the log's latest `active` or `inactive` item.
pub async fn set_last(conn: &mut PgConnection, id: AgentId, provider: &Identity, when: DateTime<Utc>) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.agents SET last_provider = $2, last_active = $3 WHERE id = $1")
        .bind(id.0)
        .bind(Json(provider))
        .bind(when)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
