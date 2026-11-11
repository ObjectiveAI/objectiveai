//! One connected tool as a list reports it.

use diverge_sdk::daemon::endpoints::connected_standalone_tools::list::server::response::ConnectedStandaloneTool;
use sqlx::PgConnection;

use super::load;
use crate::daemon::Daemon;
use crate::store;
use crate::store::connected_standalone_tools::ConnectedTool;

/// The record as a list reports it: its attachments — to agents and
/// to agent templates — loaded now, whether it is active now, and
/// whether its connect is held.
pub async fn report(conn: &mut PgConnection, daemon: &Daemon, tool: &ConnectedTool) -> Result<ConnectedStandaloneTool, store::Error> {
    let loaded = load(conn, daemon, tool).await?;
    Ok(tool.report(loaded.active, loaded.deployed, loaded.agents, loaded.templates))
}
