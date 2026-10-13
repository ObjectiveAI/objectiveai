//! The dependency tool a request names: its template's record, and
//! its run.

use std::sync::Arc;

use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use crate::containers::{ToolRun, running_dependency};
use crate::daemon::Daemon;
use crate::store::dependencies::Record;
use crate::store::{self, dependencies};

/// The template the reference names, if it is on record, and the tool
/// running from it for the reference's owner, if one does: nothing
/// for a template that is none, an owner it has no tool for, or a
/// tool that does not run. Nothing is started.
pub async fn resolve(conn: &mut PgConnection, daemon: &Daemon, reference: &reference::Dependency) -> Result<Option<(Record, Arc<ToolRun>)>, store::Error> {
    let Some(record) = dependencies::by_id(conn, &reference.template, false).await? else {
        return Ok(None);
    };
    Ok(running_dependency(daemon, conn, reference).await?.map(|run| (record, run)))
}
