//! Replacing an outgoing provider's mode.

use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::{Error, OutgoingId};

/// Write the mode, whole: the next dial presents it.
pub async fn update_mode(conn: &mut PgConnection, id: OutgoingId, mode: &Mode) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers_outgoing SET mode = $2 WHERE id = $1")
        .bind(id.0)
        .bind(Json(mode))
        .execute(&mut *conn)
        .await?;
    Ok(())
}
