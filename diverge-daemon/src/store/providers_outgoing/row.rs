//! Reading an outgoing provider off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::Outgoing;
use crate::store::{Error, OutgoingId};

/// The columns every outgoing provider query selects.
pub(super) const SELECT: &str = "SELECT id, address, mode, last_connected, tags, created, creator FROM diverge.providers_outgoing";

/// The order: oldest added first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The provider a row of [`SELECT`] holds.
pub(super) fn outgoing(row: &PgRow) -> Result<Outgoing, Error> {
    let Json(mode): Json<Mode> = row.try_get("mode")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    let last_connected: Option<DateTime<Utc>> = row.try_get("last_connected")?;
    Ok(Outgoing {
        id: OutgoingId(row.try_get("id")?),
        address: row.try_get("address")?,
        mode,
        last_connected,
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
