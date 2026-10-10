//! A volume's tags.

use std::collections::HashMap;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::reference;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// The volume's tags, sorted; none for a volume never tagged.
pub async fn of_volume(conn: &mut PgConnection, volume: &reference::Volume) -> Result<Vec<String>, Error> {
    let row = sqlx::query("SELECT tags FROM diverge.volume_tags WHERE provider = $1 AND name = $2")
        .bind(Json(&volume.provider))
        .bind(&volume.name)
        .fetch_optional(&mut *conn)
        .await?;
    Ok(match row {
        Some(row) => row.try_get("tags")?,
        None => Vec::new(),
    })
}

/// Every tagged volume's tags, by volume: one load for a whole list.
pub async fn all(conn: &mut PgConnection) -> Result<HashMap<reference::Volume, Vec<String>>, Error> {
    let rows = sqlx::query("SELECT provider, name, tags FROM diverge.volume_tags").fetch_all(&mut *conn).await?;
    let mut all = HashMap::with_capacity(rows.len());
    for row in &rows {
        let Json(provider): Json<Identity> = row.try_get("provider")?;
        let name: String = row.try_get("name")?;
        let tags: Vec<String> = row.try_get("tags")?;
        all.insert(reference::Volume { provider, name }, tags);
    }
    Ok(all)
}

/// Make the volume's tags exactly `tags`, which the caller has sorted
/// and deduplicated with [`tags::with`](crate::store::tags::with) or
/// [`tags::without`](crate::store::tags::without); none is the row
/// gone.
pub async fn set_tags(conn: &mut PgConnection, volume: &reference::Volume, tags: &[String]) -> Result<(), Error> {
    if tags.is_empty() {
        return forget(conn, volume).await;
    }
    sqlx::query(
        "INSERT INTO diverge.volume_tags (provider, name, tags) VALUES ($1, $2, $3) \
         ON CONFLICT (provider, name) DO UPDATE SET tags = EXCLUDED.tags",
    )
    .bind(Json(&volume.provider))
    .bind(&volume.name)
    .bind(tags)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// The volume is gone: its tags with it.
pub async fn forget(conn: &mut PgConnection, volume: &reference::Volume) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.volume_tags WHERE provider = $1 AND name = $2")
        .bind(Json(&volume.provider))
        .bind(&volume.name)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
