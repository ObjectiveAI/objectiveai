//! Reading a agent template off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::templates::Template;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::Record;
use crate::store::Error;

/// The columns every agent template query selects, of the live rows.
pub(super) const SELECT: &str = "SELECT id, template, tags, created, creator FROM diverge.agents_templates";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The template a row of [`SELECT`] holds.
pub(super) fn record(row: &PgRow) -> Result<Record, Error> {
    let Json(template): Json<Template> = row.try_get("template")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Record {
        id: row.try_get("id")?,
        template,
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
