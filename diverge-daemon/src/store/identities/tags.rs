//! An identity's tags.

use std::collections::HashMap;

use diverge_sdk::shared::Identity;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// The identity's tags, sorted; none for an identity never tagged.
pub async fn of_identity(conn: &mut PgConnection, identity: &Identity) -> Result<Vec<String>, Error> {
    let row = sqlx::query("SELECT tags FROM diverge.identity_tags WHERE issuer = $1 AND subject = $2")
        .bind(&identity.issuer)
        .bind(&identity.subject)
        .fetch_optional(&mut *conn)
        .await?;
    Ok(match row {
        Some(row) => row.try_get("tags")?,
        None => Vec::new(),
    })
}

/// Every tagged identity's tags, by identity: one load for a whole list.
pub async fn all(conn: &mut PgConnection) -> Result<HashMap<Identity, Vec<String>>, Error> {
    let rows = sqlx::query("SELECT issuer, subject, tags FROM diverge.identity_tags").fetch_all(&mut *conn).await?;
    let mut all = HashMap::with_capacity(rows.len());
    for row in &rows {
        let issuer: String = row.try_get("issuer")?;
        let subject: String = row.try_get("subject")?;
        let tags: Vec<String> = row.try_get("tags")?;
        all.insert(Identity { issuer, subject }, tags);
    }
    Ok(all)
}

/// Make the identity's tags exactly `tags`, which the caller has sorted
/// and deduplicated with [`tags::with`](crate::store::tags::with) or
/// [`tags::without`](crate::store::tags::without); none is the row
/// gone.
pub async fn set_tags(conn: &mut PgConnection, identity: &Identity, tags: &[String]) -> Result<(), Error> {
    if tags.is_empty() {
        sqlx::query("DELETE FROM diverge.identity_tags WHERE issuer = $1 AND subject = $2")
            .bind(&identity.issuer)
            .bind(&identity.subject)
            .execute(&mut *conn)
            .await?;
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO diverge.identity_tags (issuer, subject, tags) VALUES ($1, $2, $3) \
         ON CONFLICT (issuer, subject) DO UPDATE SET tags = EXCLUDED.tags",
    )
    .bind(&identity.issuer)
    .bind(&identity.subject)
    .bind(tags)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
