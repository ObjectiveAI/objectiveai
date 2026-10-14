//! One tool as a list reports it.

use diverge_sdk::daemon::endpoints::standalone_tools::list::server::response;
use sqlx::PgConnection;

use super::{active, agents_of, running};
use crate::daemon::Daemon;
use crate::store;
use crate::store::tools::{self, Tool};

/// The record as a list reports it: its attachments and its
/// template's mode loaded now, and whether it is active now.
pub async fn report(conn: &mut PgConnection, daemon: &Daemon, tool: &Tool) -> Result<response::Tool, store::Error> {
    let agents = agents_of(conn, tool.id).await?;
    let mode = tools::mode(conn, tool).await?;
    Ok(tool.report(active(daemon, tool.id).await, running(daemon, tool.id).await, agents, mode))
}
