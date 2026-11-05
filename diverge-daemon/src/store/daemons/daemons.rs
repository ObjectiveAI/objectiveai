//! Remote daemons' records, kept and read.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::daemons::Daemon;
use diverge_sdk::shared::Identity;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// One remote daemon's record, as kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The name.
    pub name: String,
    /// What it is.
    pub description: String,
    /// The remote, by the identity it answers as.
    pub daemon: Identity,
    /// Our account there: the identity this daemon presents.
    pub identity: Identity,
    /// Its tags, sorted.
    pub tags: Vec<String>,
    /// When it was added.
    pub created: DateTime<Utc>,
    /// Who added it.
    pub creator: Creator,
}

impl Record {
    /// The record as a get and a list report it, `connected` or not.
    pub fn report(&self, connected: bool) -> Daemon {
        Daemon {
            name: self.name.clone(),
            description: self.description.clone(),
            daemon: self.daemon.clone(),
            identity: self.identity.clone(),
            tags: self.tags.clone(),
            created: self.created,
            creator: self.creator.clone(),
            connected,
        }
    }
}

/// The columns a record is read from.
const COLUMNS: &str = "name, description, daemon_issuer, daemon_subject, identity_issuer, identity_subject, tags, created, creator";

/// A record out of its row.
fn record(row: &sqlx::postgres::PgRow) -> Result<Record, Error> {
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    Ok(Record {
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        daemon: Identity { issuer: row.try_get("daemon_issuer")?, subject: row.try_get("daemon_subject")? },
        identity: Identity { issuer: row.try_get("identity_issuer")?, subject: row.try_get("identity_subject")? },
        tags: row.try_get("tags")?,
        created: row.try_get("created")?,
        creator,
    })
}

/// Keep `record`. `false`, and nothing kept, when its name is a record's
/// already.
pub async fn create(conn: &mut PgConnection, record: &Record) -> Result<bool, Error> {
    let added = sqlx::query(
        "INSERT INTO diverge.daemons (name, description, daemon_issuer, daemon_subject, identity_issuer, identity_subject, tags, created, creator) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (name) DO NOTHING",
    )
    .bind(&record.name)
    .bind(&record.description)
    .bind(&record.daemon.issuer)
    .bind(&record.daemon.subject)
    .bind(&record.identity.issuer)
    .bind(&record.identity.subject)
    .bind(&record.tags)
    .bind(record.created)
    .bind(Json(&record.creator))
    .execute(&mut *conn)
    .await?
    .rows_affected();
    Ok(added == 1)
}

/// The record of `name`, if there is one; `lock` takes its row for the
/// transaction.
pub async fn by_name(conn: &mut PgConnection, name: &str, lock: bool) -> Result<Option<Record>, Error> {
    let lock = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("SELECT {COLUMNS} FROM diverge.daemons WHERE name = $1{lock}"))
        .bind(name)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(record).transpose()
}

/// Every record, by name.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Record>, Error> {
    let rows = sqlx::query(&format!("SELECT {COLUMNS} FROM diverge.daemons ORDER BY name")).fetch_all(&mut *conn).await?;
    rows.iter().map(record).collect()
}

/// Write `record`'s description, remote and identity over the kept one
/// of its name.
pub async fn edit(conn: &mut PgConnection, record: &Record) -> Result<(), Error> {
    sqlx::query(
        "UPDATE diverge.daemons SET description = $2, daemon_issuer = $3, daemon_subject = $4, identity_issuer = $5, identity_subject = $6 WHERE name = $1",
    )
    .bind(&record.name)
    .bind(&record.description)
    .bind(&record.daemon.issuer)
    .bind(&record.daemon.subject)
    .bind(&record.identity.issuer)
    .bind(&record.identity.subject)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Make the record's tags exactly `tags`, sorted and deduplicated by
/// the caller.
pub async fn set_tags(conn: &mut PgConnection, name: &str, tags: &[String]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.daemons SET tags = $2 WHERE name = $1")
        .bind(name)
        .bind(tags)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Delete the record of `name`.
pub async fn delete(conn: &mut PgConnection, name: &str) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.daemons WHERE name = $1").bind(name).execute(&mut *conn).await?;
    Ok(())
}

/// Whether a connected tool names the record of `name`: what refuses
/// its delete.
pub async fn in_use(conn: &mut PgConnection, name: &str) -> Result<bool, Error> {
    let row = sqlx::query("SELECT EXISTS (SELECT 1 FROM diverge.tools WHERE connected_daemon = $1) AS used")
        .bind(name)
        .fetch_one(&mut *conn)
        .await?;
    Ok(row.try_get("used")?)
}
