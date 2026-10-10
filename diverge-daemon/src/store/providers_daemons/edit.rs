//! Replacing a daemon record's mode and links.

use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::{DaemonId, Error};

/// Write the mode and the links, whole: the next connection presents
/// the mode, through the links. The caller has every link's provider
/// checked to be on record.
pub async fn update(conn: &mut PgConnection, id: DaemonId, mode: &Mode, links: &[Link]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers_daemons SET mode = $2, links = $3 WHERE id = $1")
        .bind(id.0)
        .bind(Json(mode))
        .bind(Json(links))
        .execute(&mut *conn)
        .await?;
    Ok(())
}
