//! Reading a connected tool off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::reference;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::ConnectedTool;
use crate::store::{ConnectedToolId, Error, daemons};

/// The columns every connected tool query selects.
pub(super) const SELECT: &str = "SELECT id, daemon, route, tool, index, name, last_deployed, last_active, tags, created, creator \
     FROM diverge.connected_standalone_tools";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The tool a row of [`SELECT`] holds.
pub(super) fn tool(row: &PgRow) -> Result<ConnectedTool, Error> {
    let route: Vec<i64> = row.try_get("route")?;
    let Json(tool): Json<reference::Tool> = row.try_get("tool")?;
    let index: i64 = row.try_get("index")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(ConnectedTool {
        id: ConnectedToolId(row.try_get("id")?),
        daemon: daemons::from_column(row.try_get("daemon")?),
        route: route.into_iter().map(daemons::from_column).collect(),
        tool,
        index: u64::try_from(index).unwrap_or(0),
        name: row.try_get("name")?,
        last_deployed: row.try_get("last_deployed")?,
        last_active: row.try_get("last_active")?,
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
