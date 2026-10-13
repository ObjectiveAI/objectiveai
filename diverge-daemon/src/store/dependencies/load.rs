//! Loading a dependency template's body.

use diverge_sdk::shared::containers::dependencies::Template;
use sqlx::{PgConnection, Row as _};
use sqlx::types::Json;

use crate::store::Error;

/// The body under the id, if any.
pub async fn by_id(conn: &mut PgConnection, id: &str) -> Result<Option<Template>, Error> {
    let row = sqlx::query("SELECT template FROM diverge.dependencies WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    match row {
        Some(row) => {
            let Json(template): Json<Template> = row.try_get("template")?;
            Ok(Some(template))
        }
        None => Ok(None),
    }
}
