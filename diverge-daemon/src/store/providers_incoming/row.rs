//! Reading an incoming credential off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::Incoming;
use crate::store::{Error, IncomingId};

/// The columns every incoming credential query selects.
pub(super) const SELECT: &str = "SELECT id, identity, address, key_hash, created, creator FROM diverge.providers_incoming";

/// The order: oldest added first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The credential a row of [`SELECT`] holds.
pub(super) fn incoming(row: &PgRow) -> Result<Incoming, Error> {
    let address: Option<String> = row.try_get("address")?;
    let address = match address {
        Some(value) => Some(value.parse().map_err(|source| Error::Address {
            value: value.clone(),
            source,
        })?),
        None => None,
    };
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Incoming {
        id: IncomingId(row.try_get("id")?),
        identity: row.try_get("identity")?,
        address,
        key_hash: row.try_get("key_hash")?,
        created,
        creator,
    })
}
