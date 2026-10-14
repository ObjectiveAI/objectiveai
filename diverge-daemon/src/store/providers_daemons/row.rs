//! Reading a daemon record off a row, and its accounts off theirs.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::{Account, Record};
use crate::store::{DaemonId, Error};

/// The columns a record is read from.
pub(super) const SELECT: &str = "SELECT id, name, links, tags, created, creator FROM diverge.providers_daemons";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The record a row of [`SELECT`] holds, with no accounts yet: the
/// loader fills them.
pub(super) fn record(row: &PgRow) -> Result<Record, Error> {
    let Json(links): Json<Vec<Link>> = row.try_get("links")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Record {
        id: DaemonId(row.try_get("id")?),
        name: row.try_get("name")?,
        links,
        accounts: Vec::new(),
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}

/// The columns an account is read from, with the daemon it is on.
pub(super) const SELECT_ACCOUNT: &str = "SELECT daemon, name, mode, created FROM diverge.providers_daemons_accounts";

/// The order of accounts: oldest first.
pub(super) const ORDER_ACCOUNT: &str = "ORDER BY created, name";

/// The account a row of [`SELECT_ACCOUNT`] holds, and whose it is.
pub(super) fn account(row: &PgRow) -> Result<(DaemonId, Account), Error> {
    let Json(mode): Json<Mode> = row.try_get("mode")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok((
        DaemonId(row.try_get("daemon")?),
        Account {
            name: row.try_get("name")?,
            mode,
            created,
        },
    ))
}
