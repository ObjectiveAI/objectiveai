//! Providers' records, kept and read.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::Direction;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::description::Description;
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
    /// What the daemon announces itself with on it: present, the
    /// daemon holds its `daemons::announce` open with this text while
    /// connected; absent, it does not announce itself there.
    pub announcement: Option<Description>,
    /// Whether the daemon accepts other daemons through it: holds its
    /// `daemons::accept` open while connected.
    pub accept_daemons: bool,
    /// Whether the daemon listens for messages on it: holds its
    /// `daemons::listen` open while connected.
    pub messageable: bool,
    /// Whether the daemon works with it.
    pub enabled: bool,
    /// What it is, in the operator's words: the provider never sees it.
    pub description: String,
    /// Its tags, sorted.
    pub tags: Vec<String>,
    /// When it was added.
    pub created: DateTime<Utc>,
    /// Who added it.
    pub creator: Creator,
}

/// The columns a record is read from.
const COLUMNS: &str = "issuer, subject, direction, announcement, accept_daemons, messageable, enabled, description, tags, created, creator";

/// A record out of its row. An announcement kept that is no
/// [`Description`] — which only a hand on the table makes — is the
/// row's failure, as a direction that does not parse is.
fn record(row: &sqlx::postgres::PgRow) -> Result<Record, Error> {
    let Json(direction): Json<Direction> = row.try_get("direction")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let announcement: Option<String> = row.try_get("announcement")?;
    let announcement = announcement.map(Description::new).transpose().map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    Ok(Record {
        identity: Identity { issuer: row.try_get("issuer")?, subject: row.try_get("subject")? },
        direction,
        announcement,
        accept_daemons: row.try_get("accept_daemons")?,
        messageable: row.try_get("messageable")?,
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
        "INSERT INTO diverge.providers (issuer, subject, direction, announcement, accept_daemons, messageable, enabled, description, tags, created, creator) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) ON CONFLICT (issuer, subject) DO NOTHING",
    )
    .bind(&record.identity.issuer)
    .bind(&record.identity.subject)
    .bind(Json(&record.direction))
    .bind(record.announcement.as_ref().map(Description::as_str))
    .bind(record.accept_daemons)
    .bind(record.messageable)
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

/// Whether `identity` has a record, its row held shared for the
/// transaction: a pin checked so is never stranded, since a delete
/// waits for the row and then finds the pin.
pub async fn share(conn: &mut PgConnection, identity: &Identity) -> Result<bool, Error> {
    let row = sqlx::query("SELECT 1 FROM diverge.providers WHERE issuer = $1 AND subject = $2 FOR SHARE")
        .bind(&identity.issuer)
        .bind(&identity.subject)
        .fetch_optional(&mut *conn)
        .await?;
    Ok(row.is_some())
}

/// Every record, oldest first: the order the daemon tries providers in
/// when nothing pins a run.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Record>, Error> {
    let rows = sqlx::query(&format!("SELECT {COLUMNS} FROM diverge.providers ORDER BY created, issuer, subject"))
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(record).collect()
}

/// Write `record`'s direction — of which an edit changes only an
/// incoming one's `ips` — its announcement, whether it accepts
/// daemons, whether it is messageable, and its description over the
/// kept one of its identity.
pub async fn edit(conn: &mut PgConnection, record: &Record) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.providers SET direction = $3, announcement = $4, accept_daemons = $5, messageable = $6, description = $7 WHERE issuer = $1 AND subject = $2")
        .bind(&record.identity.issuer)
        .bind(&record.identity.subject)
        .bind(Json(&record.direction))
        .bind(record.announcement.as_ref().map(Description::as_str))
        .bind(record.accept_daemons)
        .bind(record.messageable)
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
