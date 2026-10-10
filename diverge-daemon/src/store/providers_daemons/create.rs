//! Adding a daemon record.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{DaemonId, Error};

/// What a new daemon record is made of: every link's provider checked
/// by the caller to be on record.
#[derive(Debug, Clone)]
pub struct New {
    /// The name, as given.
    pub name: String,
    /// How this daemon authenticates there.
    pub mode: Mode,
    /// The providers it is reached through.
    pub links: Vec<Link>,
    /// Who added it.
    pub creator: Creator,
}

/// What an add came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The record, added.
    Created(DaemonId),
    /// The name is another record's already; nothing was added.
    Exists,
}

/// Insert the record.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let inserted = sqlx::query("INSERT INTO diverge.providers_daemons (name, mode, links, creator) VALUES ($1, $2, $3, $4) RETURNING id")
        .bind(&new.name)
        .bind(Json(&new.mode))
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
