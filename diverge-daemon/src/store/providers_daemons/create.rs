//! Adding a daemon record.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{DaemonId, Error};

/// What an add writes: the name, the links, and who added it. The
/// accounts come after, one by one.
#[derive(Debug, Clone)]
pub struct New {
    /// The name.
    pub name: String,
    /// The links.
    pub links: Vec<Link>,
    /// Who adds it.
    pub creator: Creator,
}

/// An add's outcome: the record made, or the name taken already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// Made, with its row.
    Created(DaemonId),
    /// The name is a record's already; nothing was made.
    Exists,
}

/// Insert the record, holding no account and no tag yet.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let inserted = sqlx::query("INSERT INTO diverge.providers_daemons (name, links, creator) VALUES ($1, $2, $3) RETURNING id")
        .bind(&new.name)
        .bind(Json(&new.links))
        .bind(Json(&new.creator))
        .fetch_one(&mut *conn)
        .await;
    match inserted {
        Ok(row) => Ok(Created::Created(DaemonId(row.try_get("id")?))),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
