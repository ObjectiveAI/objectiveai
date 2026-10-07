//! Reading a resource off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::resources::Kind;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::Record;
use crate::store::Error;

/// The columns every resource query selects.
pub(super) const SELECT: &str = "SELECT id, kind, description, bytes, tags, created, creator FROM diverge.resources";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The resource a row of [`SELECT`] holds.
pub(super) fn record(row: &PgRow) -> Result<Record, Error> {
    let kind: String = row.try_get("kind")?;
    let kind = match kind.as_str() {
        "file" => Kind::File,
        "directory" => Kind::Directory,
        other => {
            return Err(Error::Json(serde::de::Error::custom(format!(
                "`{other}` is not a resource kind"
            ))));
        }
    };
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    let bytes: i64 = row.try_get("bytes")?;
    Ok(Record {
        id: row.try_get("id")?,
        kind,
        description: row.try_get("description")?,
        bytes: u64::try_from(bytes).unwrap_or(0),
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
