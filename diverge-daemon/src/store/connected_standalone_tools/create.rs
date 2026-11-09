//! Holding another daemon's tool.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::daemons::DaemonId;
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::canonical;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{ConnectedToolId, Error, counters, daemons, tool_names};

/// What a new connected tool is made of, checked by the caller — the
/// remote daemon's record there, its `connect` granted.
#[derive(Debug, Clone)]
pub struct New {
    /// The remote daemon the tool is on, by the ID of this daemon's
    /// record of it.
    pub daemon: DaemonId,
    /// The records the connect is relayed through beyond `daemon`.
    pub route: Vec<DaemonId>,
    /// The tool, as that daemon names it.
    pub tool: reference::Tool,
    /// The name, if any.
    pub name: Option<String>,
    /// Who makes it.
    pub creator: Creator,
}

/// What a create came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The tool, held, with the index it was given.
    Created(ConnectedToolId, u64),
    /// The name is a standalone or a connected tool's already; nothing
    /// was held.
    Exists,
}

/// The counter a connected tool's index is the next of: one per daemon
/// record and tool there, `tools:<daemon>:<canonical tool JSON>` — the
/// key connected tools have always counted under, so no number given
/// to one since deleted is given again.
fn counter(daemon: DaemonId, tool: &reference::Tool) -> Result<String, Error> {
    let tool = canonical::bytes(tool).map_err(Error::Json)?;
    Ok(format!("tools:{daemon}:{}", String::from_utf8_lossy(&tool)))
}

/// Insert the tool, its index the next among the connected tools held
/// from its daemon's tool, holding no tag yet, its name claimed across
/// both tool families as [`tool_names`] states. The counter is advanced
/// in the caller's transaction, so a create that is not committed
/// gives the number back.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    if tool_names::claim(conn, new.name.as_deref(), None).await? {
        return Ok(Created::Exists);
    }
    let index = counters::next(conn, &counter(new.daemon, &new.tool)?).await?;
    let route: Vec<i64> = new.route.iter().copied().map(daemons::to_column).collect();
    let inserted = sqlx::query(
        "INSERT INTO diverge.connected_standalone_tools (daemon, route, tool, index, name, creator) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(daemons::to_column(new.daemon))
    .bind(route)
    .bind(Json(&new.tool))
    .bind(i64::try_from(index).unwrap_or(i64::MAX))
    .bind(&new.name)
    .bind(Json(&new.creator))
    .fetch_one(&mut *conn)
    .await;
    match inserted {
        Ok(row) => Ok(Created::Created(ConnectedToolId(row.try_get("id")?), index)),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
