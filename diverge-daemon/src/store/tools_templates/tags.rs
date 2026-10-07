//! A tool template's tags.

use sqlx::PgConnection;

use crate::store::Error;

/// Make the template's tags exactly `tags`, which the caller has
/// sorted and deduplicated with [`tags::with`](crate::store::tags::with)
/// or [`tags::without`](crate::store::tags::without).
pub async fn set_tags(conn: &mut PgConnection, id: &str, tags: &[String]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.tools_templates SET tags = $2 WHERE id = $1")
        .bind(id)
        .bind(tags)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
