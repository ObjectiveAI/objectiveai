//! A daemon record's tags.

use sqlx::PgConnection;

use crate::store::{DaemonId, Error};

/// Make the record's tags exactly `tags`, which the caller has sorted
/// and deduplicated with [`tags::with`](crate::store::tags::with) or
/// [`tags::without`](crate::store::tags::without).
pub async fn set_tags(conn: &mut PgConnection, id: DaemonId, tags: &[String]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers_daemons SET tags = $2 WHERE id = $1")
        .bind(id.0)
        .bind(tags)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
