//! Reading a daemon record off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::Record;
use crate::store::{DaemonId, Error};

/// The columns every daemon record query selects.
pub(super) const SELECT: &str = "SELECT id, name, mode, links, tags, created, creator FROM diverge.providers_daemons";

/// The order: oldest added first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The record a row of [`SELECT`] holds.
pub(super) fn record(row: &PgRow) -> Result<Record, Error> {
    let Json(mode): Json<Mode> = row.try_get("mode")?;
    let Json(links): Json<Vec<Link>> = row.try_get("links")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Record {
        id: DaemonId(row.try_get("id")?),
        name: row.try_get("name")?,
        mode,
        links,
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
