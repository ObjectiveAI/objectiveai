//! Providers' records, kept and read.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::Direction;
use diverge_sdk::shared::Identity;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// One provider's record, as kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The provider, by the identity its token establishes.
    pub identity: Identity,
    /// How its connection is made.
    pub direction: Direction,
    /// Whether the daemon is discoverable to it.
    pub discover: bool,
    /// Whether the daemon works with it.
    pub enabled: bool,
    /// What it is.
    pub description: String,
    /// Its tags, sorted.
    pub tags: Vec<String>,
    /// When it was added.
    pub created: DateTime<Utc>,
    /// Who added it.
    pub creator: Creator,
}

/// The columns a record is read from.
const COLUMNS: &str = "issuer, subject, direction, discover, enabled, description, tags, created, creator";

/// A record out of its row.
fn record(row: &sqlx::postgres::PgRow) -> Result<Record, Error> {
    let Json(direction): Json<Direction> = row.try_get("direction")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    Ok(Record {
        identity: Identity { issuer: row.try_get("issuer")?, subject: row.try_get("subject")? },
        direction,
        discover: row.try_get("discover")?,
        enabled: row.try_get("enabled")?,
        description: row.try_get("description")?,
        tags: row.try_get("tags")?,
        created: row.try_get("created")?,
        creator,
    })
}

/// Keep `record`. `false`, and nothing kept, when its identity is a
/// record's already.
pub async fn create(conn: &mut PgConnection, record: &Record) -> Result<bool, Error> {
    let added = sqlx::query(
        "INSERT INTO diverge.providers (issuer, subject, direction, discover, enabled, description, tags, created, creator) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (issuer, subject) DO NOTHING",
    )
    .bind(&record.identity.issuer)
    .bind(&record.identity.subject)
    .bind(Json(&record.direction))
    .bind(record.discover)
    .bind(record.enabled)
    .bind(&record.description)
    .bind(&record.tags)
    .bind(record.created)
    .bind(Json(&record.creator))
    .execute(&mut *conn)
    .await?
    .rows_affected();
    Ok(added == 1)
}

/// The record of `identity`, if there is one; `lock` takes its row for
/// the transaction.
pub async fn by_identity(conn: &mut PgConnection, identity: &Identity, lock: bool) -> Result<Option<Record>, Error> {
    let lock = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("SELECT {COLUMNS} FROM diverge.providers WHERE issuer = $1 AND subject = $2{lock}"))
        .bind(&identity.issuer)
        .bind(&identity.subject)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(record).transpose()
}

/// Every record, oldest first: the order the daemon tries providers in
/// when nothing pins a run.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Record>, Error> {
    let rows = sqlx::query(&format!("SELECT {COLUMNS} FROM diverge.providers ORDER BY created, issuer, subject"))
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(record).collect()
}

/// Write `record`'s direction, discover flag and description over the
/// kept one of its identity.
pub async fn edit(conn: &mut PgConnection, record: &Record) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers SET direction = $3, discover = $4, description = $5 WHERE issuer = $1 AND subject = $2")
        .bind(&record.identity.issuer)
        .bind(&record.identity.subject)
        .bind(Json(&record.direction))
        .bind(record.discover)
        .bind(&record.description)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Make the record enabled, or not.
pub async fn set_enabled(conn: &mut PgConnection, identity: &Identity, enabled: bool) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers SET enabled = $3 WHERE issuer = $1 AND subject = $2")
        .bind(&identity.issuer)
        .bind(&identity.subject)
        .bind(enabled)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Make the record's tags exactly `tags`, sorted and deduplicated by
/// the caller.
pub async fn set_tags(conn: &mut PgConnection, identity: &Identity, tags: &[String]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers SET tags = $3 WHERE issuer = $1 AND subject = $2")
        .bind(&identity.issuer)
        .bind(&identity.subject)
        .bind(tags)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Delete the record of `identity`, and the tags kept on its volumes.
pub async fn delete(conn: &mut PgConnection, identity: &Identity) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.volume_tags WHERE provider = $1").bind(Json(identity)).execute(&mut *conn).await?;
    sqlx::query("DELETE FROM diverge.providers WHERE issuer = $1 AND subject = $2")
        .bind(&identity.issuer)
        .bind(&identity.subject)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Whether an agent or a tool is pinned to `identity`'s provider, or
/// mounts a volume of it: what refuses its delete.
pub async fn in_use(conn: &mut PgConnection, identity: &Identity) -> Result<bool, Error> {
    let mounts = serde_json::json!([{ "provider": identity }]);
    let row = sqlx::query(
        "SELECT EXISTS ( \
             SELECT 1 FROM diverge.agents WHERE provider->'identity' = $1 OR fuse_file_mounts @> $2 OR fuse_directory_mounts @> $2 \
             UNION ALL \
             SELECT 1 FROM diverge.tools WHERE provider->'identity' = $1 OR fuse_file_mounts @> $2 OR fuse_directory_mounts @> $2 \
         ) AS used",
    )
    .bind(Json(identity))
    .bind(Json(&mounts))
    .fetch_one(&mut *conn)
    .await?;
    Ok(row.try_get("used")?)
}
