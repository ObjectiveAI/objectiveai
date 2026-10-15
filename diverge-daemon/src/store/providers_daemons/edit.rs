//! Replacing a daemon record's links.

use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::{DaemonId, Error};

/// Replace the links whole.
pub async fn update(conn: &mut PgConnection, id: DaemonId, links: &[Link]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers_daemons SET links = $2 WHERE id = $1")
        .bind(id.0)
        .bind(Json(links))
        .execute(&mut *conn)
        .await?;
    Ok(())
}
