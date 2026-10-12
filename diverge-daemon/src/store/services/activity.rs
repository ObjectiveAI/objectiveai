//! Where a service last ran.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::{Error, ServiceId};

/// Keep that the service's container began or ceased running on
/// `provider`.
pub async fn set_last(conn: &mut PgConnection, id: ServiceId, provider: &Identity) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.services SET last_provider = $2 WHERE id = $1")
        .bind(id.0)
        .bind(Json(provider))
        .execute(&mut *conn)
        .await?;
    Ok(())
}
