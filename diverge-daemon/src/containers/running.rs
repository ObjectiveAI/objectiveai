//! The dependency tool a reference names, if it runs.

use std::sync::Arc;

use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use super::{ToolKey, ToolRun};
use crate::daemon::Daemon;
use crate::store::{self, dependency_tools};

/// The run of the tool the reference names, if the tool is on record
/// and its container is up: a template that is none, an owner the
/// template has no tool under, and a tool that does not run are each
/// `None`. Nothing is started.
pub async fn running_dependency(daemon: &Daemon, conn: &mut PgConnection, reference: &reference::Dependency) -> Result<Option<Arc<ToolRun>>, store::Error> {
    let Some(tool) = dependency_tools::by_reference(conn, reference, false).await? else {
        return Ok(None);
    };
    Ok(daemon.live.tool_run(ToolKey::Dependency(tool.id)).await)
}
