//! Putting an image's answer on record.

use diverge_sdk::shared::containers::info::Document;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::Error;

/// Record that the image under the digest ships `document`. One on
/// record already is left as it is, since the same digest is the same
/// image and so the same answer.
pub async fn save(conn: &mut PgConnection, digest: &str, document: &Document) -> Result<(), Error> {
    sqlx::query("INSERT INTO diverge.image_info (digest, document) VALUES ($1, $2) ON CONFLICT (digest) DO NOTHING")
        .bind(digest)
        .bind(Json(document))
        .execute(&mut *conn)
        .await?;
    Ok(())
}
