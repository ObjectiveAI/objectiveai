//! Taking a route up.

use diverge_sdk::daemon::endpoints::tools::routes::Path;
use sqlx::PgConnection;

use crate::store::Error;

/// Delete the route at the position. Whether a container is served
/// through it now is the caller's to have checked.
pub async fn delete(conn: &mut PgConnection, path: &Path) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.routes WHERE agent = $1 AND templates = $2")
        .bind(&path.agent)
        .bind(&path.templates)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
