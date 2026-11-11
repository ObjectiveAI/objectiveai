//! Reading an image's answer off the record.

use diverge_sdk::shared::containers::info::Document;
use sqlx::{PgConnection, Row as _};
use sqlx::types::Json;

use crate::store::Error;

/// What is on record for the digest: `None` when nothing is;
/// `Some(Some(document))` when the image ships that document;
/// `Some(None)` when the image ships none.
pub async fn by_digest(conn: &mut PgConnection, digest: &str) -> Result<Option<Option<Document>>, Error> {
    let row = sqlx::query("SELECT document FROM diverge.image_info WHERE digest = $1")
        .bind(digest)
        .fetch_optional(&mut *conn)
        .await?;
    match row {
        Some(row) => {
            let document: Option<Json<Document>> = row.try_get("document")?;
            Ok(Some(document.map(|Json(document)| document)))
        }
        None => Ok(None),
    }
}
