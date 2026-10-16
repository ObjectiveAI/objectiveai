//! A vault key's tags.

use sqlx::PgConnection;

use crate::store::Error;

/// Make the key's tags exactly `tags`, which the caller has sorted
/// and deduplicated with [`tags::with`](crate::store::tags::with) or
/// [`tags::without`](crate::store::tags::without).
pub async fn set_tags(conn: &mut PgConnection, key: &str, tags: &[String]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.vault SET tags = $2 WHERE key = $1")
        .bind(key)
        .bind(tags)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
