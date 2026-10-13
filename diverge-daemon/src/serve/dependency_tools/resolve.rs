//! The dependency tool a request names: its row, and its run.

use std::sync::Arc;

use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use crate::containers::{ToolKey, ToolRun};
use crate::daemon::Daemon;
use crate::store::dependency_tools::DependencyTool;
use crate::store::{self, dependency_tools};

/// The tool the reference names, if it is on record — its template
/// under its owner — with its run, if its container is up: nothing
/// for a template that is none or an owner it has no tool under.
/// With `lock`, the row is locked for the transaction. Nothing is
/// started.
pub async fn resolve(
    conn: &mut PgConnection,
    daemon: &Daemon,
    reference: &reference::Dependency,
    lock: bool,
) -> Result<Option<(DependencyTool, Option<Arc<ToolRun>>)>, store::Error> {
    let Some(tool) = dependency_tools::by_reference(conn, reference, lock).await? else {
        return Ok(None);
    };
    let run = daemon.live.tool_run(ToolKey::Dependency(tool.id)).await;
    Ok(Some((tool, run)))
}
