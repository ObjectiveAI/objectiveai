//! Reading a route off a row.

use diverge_sdk::daemon::creator::Creator;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::Route;
use crate::store::{Error, ToolId};

/// The columns every route query selects.
pub(super) const SELECT: &str = "SELECT agent, templates, tool, created, creator FROM diverge.routes";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, agent, templates";

/// The route a row of [`SELECT`] holds.
pub(super) fn route(row: &PgRow) -> Result<Route, Error> {
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    Ok(Route {
        agent: row.try_get("agent")?,
        templates: row.try_get("templates")?,
        tool: ToolId(row.try_get("tool")?),
        created: row.try_get("created")?,
        creator,
    })
}
