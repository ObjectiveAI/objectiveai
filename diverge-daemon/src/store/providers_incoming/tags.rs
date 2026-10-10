//! An incoming credential's tags.

use sqlx::PgConnection;

use crate::store::{Error, IncomingId};

/// Make the credential's tags exactly `tags`, which the caller has
/// sorted and deduplicated with [`tags::with`](crate::store::tags::with) or
/// [`tags::without`](crate::store::tags::without).
pub async fn set_tags(conn: &mut PgConnection, id: IncomingId, tags: &[String]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers_incoming SET tags = $2 WHERE id = $1")
        .bind(id.0)
        .bind(tags)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
