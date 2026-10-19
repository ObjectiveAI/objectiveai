//! A dependency tool's tags.

use sqlx::PgConnection;

use crate::store::{DependencyToolId, Error};

/// Make the tool's tags exactly `tags`, which the caller has sorted
/// and deduplicated with [`tags::with`](crate::store::tags::with) or
/// [`tags::without`](crate::store::tags::without).
pub async fn set_tags(conn: &mut PgConnection, id: DependencyToolId, tags: &[String]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.dependency_tools SET tags = $2 WHERE id = $1")
        .bind(id.0)
        .bind(tags)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
