//! Putting a dependency template's body on record.

use diverge_sdk::shared::containers::dependencies::Template;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::Error;

/// Insert the body under the id the caller hashed it to; one there
/// already is left as it is, since the same hash is the same body.
pub async fn create(conn: &mut PgConnection, id: &str, template: &Template) -> Result<(), Error> {
    sqlx::query("INSERT INTO diverge.dependencies (id, template) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING")
        .bind(id)
        .bind(Json(template))
        .execute(&mut *conn)
        .await?;
    Ok(())
}
