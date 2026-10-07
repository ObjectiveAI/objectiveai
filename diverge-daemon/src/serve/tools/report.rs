//! One tool as a list reports it.

use diverge_sdk::daemon::endpoints::tools::list::server::response;
use sqlx::PgConnection;

use super::{active, agents_of, running};
use crate::daemon::Daemon;
use crate::store::tools::{Tool, admissions};
use crate::store::{self, routes};

/// The tool as a list reports it: its attachments, the positions
/// routed to it and its admissions loaded now, and whether it is
/// active now. A connected tool reports no admissions: its runner
/// admits.
pub async fn report(conn: &mut PgConnection, daemon: &Daemon, tool: &Tool) -> Result<response::Tool, store::Error> {
    let agents = agents_of(conn, tool.id).await?;
    let routes = routes::of_tool(conn, tool.id).await?;
    let admissions = if tool.is_connected() {
        Vec::new()
    } else {
        admissions::of_tool(conn, tool.id)
            .await?
            .into_iter()
            .map(|record| record.admission)
            .collect()
    };
    Ok(tool.report(active(daemon, tool.id).await, running(daemon, tool.id).await, agents, routes, admissions))
}
