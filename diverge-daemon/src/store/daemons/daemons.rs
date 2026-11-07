//! Remote daemons' records, kept and read.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::daemons::{Daemon, DaemonId, Routes};
use diverge_sdk::shared::Identity;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// One remote daemon's record, as kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The ID: the hash of `daemon`, `identity` and `routes`.
    pub id: DaemonId,
    /// What it is.
    pub description: String,
    /// The remote, by the identity it answers as.
    pub daemon: Identity,
    /// Our account there: the identity this daemon presents.
    pub identity: Identity,
    /// The routes it may be reached by.
    pub routes: Routes,
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
            id: self.id,
            description: self.description.clone(),
            daemon: self.daemon.clone(),
            identity: self.identity.clone(),
            routes: self.routes.clone(),
            tags: self.tags.clone(),
            created: self.created,
            creator: self.creator.clone(),
            connected,
        }
    }
}

/// What a new record is made of: everything of a [`Record`] but its ID,
/// which [`create`] computes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct New {
    /// What it is.
    pub description: String,
    /// The remote, by the identity it answers as.
    pub daemon: Identity,
    /// Our account there: the identity this daemon presents.
    pub identity: Identity,
    /// The routes it may be reached by.
    pub routes: Routes,
    /// Its tags, sorted.
    pub tags: Vec<String>,
    /// When it is added.
    pub created: DateTime<Utc>,
    /// Who adds it.
    pub creator: Creator,
}

/// An ID as its column holds it: the `u64` bit-cast to the `BIGINT`
/// Postgres has, so every ID has one column value, and back by
/// [`from_column`].
pub fn to_column(id: DaemonId) -> i64 {
    id.0 as i64
}

/// An ID out of its column: the inverse of [`to_column`].
pub fn from_column(column: i64) -> DaemonId {
    DaemonId(column as u64)
}

/// The columns a record is read from.
const COLUMNS: &str = "id, description, daemon_issuer, daemon_subject, identity_issuer, identity_subject, providers, ips, addresses, tags, created, creator";

/// A record out of its row.
fn record(row: &sqlx::postgres::PgRow) -> Result<Record, Error> {
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let providers: Option<Json<_>> = row.try_get("providers")?;
    let ips: Option<Json<_>> = row.try_get("ips")?;
    let addresses: Option<Json<_>> = row.try_get("addresses")?;
    let routes = Routes {
        providers: providers.map(|Json(providers)| providers),
        ips: ips.map(|Json(ips)| ips),
        addresses: addresses.map(|Json(addresses)| addresses),
    };
    Ok(Record {
        id: from_column(row.try_get("id")?),
        description: row.try_get("description")?,
        daemon: Identity { issuer: row.try_get("daemon_issuer")?, subject: row.try_get("daemon_subject")? },
        identity: Identity { issuer: row.try_get("identity_issuer")?, subject: row.try_get("identity_subject")? },
        routes,
        tags: row.try_get("tags")?,
        created: row.try_get("created")?,
        creator,
    })
}

/// Keep `new` under the ID of what it reaches and as whom —
/// [`DaemonId::of`] — and hand the ID back, `true` beside it; `false`,
/// and nothing kept, when a record of that ID is kept already: the same
/// remote, as the same account, by the same routes.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<(DaemonId, bool), Error> {
    let id = DaemonId::of(&new.daemon, &new.identity, &new.routes);
    let added = sqlx::query(
        "INSERT INTO diverge.daemons (id, description, daemon_issuer, daemon_subject, identity_issuer, identity_subject, providers, ips, addresses, tags, created, creator) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) ON CONFLICT (id) DO NOTHING",
    )
    .bind(to_column(id))
    .bind(&new.description)
    .bind(&new.daemon.issuer)
    .bind(&new.daemon.subject)
    .bind(&new.identity.issuer)
    .bind(&new.identity.subject)
    .bind(new.routes.providers.as_ref().map(Json))
    .bind(new.routes.ips.as_ref().map(Json))
    .bind(new.routes.addresses.as_ref().map(Json))
    .bind(&new.tags)
    .bind(new.created)
    .bind(Json(&new.creator))
    .execute(&mut *conn)
    .await?
    .rows_affected();
    Ok((id, added == 1))
}

/// The record of `id`, if there is one; `lock` takes its row for the
/// transaction.
pub async fn by_id(conn: &mut PgConnection, id: DaemonId, lock: bool) -> Result<Option<Record>, Error> {
    let lock = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("SELECT {COLUMNS} FROM diverge.daemons WHERE id = $1{lock}"))
        .bind(to_column(id))
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(record).transpose()
}

/// Every record, oldest first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Record>, Error> {
    let rows = sqlx::query(&format!("SELECT {COLUMNS} FROM diverge.daemons ORDER BY created, id")).fetch_all(&mut *conn).await?;
    rows.iter().map(record).collect()
}

/// Write `description` over the kept record of `id`: the one member of a
/// record an edit changes, since the rest is its ID.
pub async fn edit(conn: &mut PgConnection, id: DaemonId, description: &str) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.daemons SET description = $2 WHERE id = $1")
        .bind(to_column(id))
        .bind(description)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Make the record's tags exactly `tags`, sorted and deduplicated by
/// the caller.
pub async fn set_tags(conn: &mut PgConnection, id: DaemonId, tags: &[String]) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.daemons SET tags = $2 WHERE id = $1")
        .bind(to_column(id))
        .bind(tags)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Delete the record of `id`.
pub async fn delete(conn: &mut PgConnection, id: DaemonId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.daemons WHERE id = $1").bind(to_column(id)).execute(&mut *conn).await?;
    Ok(())
}

/// Whether a connected tool names the record of `id`: what refuses its
/// delete.
pub async fn in_use(conn: &mut PgConnection, id: DaemonId) -> Result<bool, Error> {
    let row = sqlx::query("SELECT EXISTS (SELECT 1 FROM diverge.tools WHERE connected_daemon = $1) AS used")
        .bind(to_column(id))
        .fetch_one(&mut *conn)
        .await?;
    Ok(row.try_get("used")?)
}
