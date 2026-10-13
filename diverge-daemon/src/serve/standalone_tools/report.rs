//! One tool as a list reports it.

use diverge_sdk::daemon::endpoints::standalone_tools::list::server::response;
use sqlx::PgConnection;

use super::{active, agents_of, running};
use crate::daemon::Daemon;
use crate::store::tools::Tool;
use crate::store;

/// The record as a list reports it: its attachments loaded now, and
/// whether it is active now.
pub async fn report(conn: &mut PgConnection, daemon: &Daemon, tool: &Tool) -> Result<response::Tool, store::Error> {
    let agents = agents_of(conn, tool.id).await?;
    Ok(tool.report(active(daemon, tool.id).await, running(daemon, tool.id).await, agents))
}
