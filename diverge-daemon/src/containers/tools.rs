//! One container per tool on record: started for the first thing that
//! uses it, held up by what is in flight into it and the connect
//! scopes on it, and — a transient tool — stopped by its idle clock
//! `idle_seconds` after the last of those left; a service is kept
//! while anything is attached to it and retired by its own clock.

use std::collections::HashSet;
use std::sync::Arc;

use super::{ConnectionId, StartError, ToolKey, ToolRun, services, start};
use crate::daemon::Daemon;
use crate::store::tools::Tool;

/// The tool's run: started now if it was not running, and touched if
/// it was. Nothing is held by this: whoever uses the run takes an
/// [`Inflight`](super::Inflight) guard for the use, and a connect
/// scope [`connect`]s. A service's run is kept from here on — its
/// supervisor and its clock started if they were not — and told what
/// holds it.
pub async fn use_tool(daemon: &Arc<Daemon>, tool: &Tool) -> Result<Arc<ToolRun>, StartError> {
    let run = match daemon.live.tool_run(ToolKey::Record(tool.id)).await {
        Some(run) => {
            run.touch();
            run
        }
        None => start::tool(daemon, tool).await?,
    };
    if run.service {
        services::keep(daemon, run.id).await;
        services::held(daemon, run.id).await;
    }
    Ok(run)
}

/// A connect scope of another daemon's holds the run from now on,
/// until [`disconnect`]: the run is told it is held.
pub async fn connect(run: &ToolRun, connection: ConnectionId) {
    let mut connections = run.connections.lock().await;
    connections.insert(connection);
    held(run, &connections);
}

/// The connect scope is over: the run is told whether anything holds
/// it still, and a service told its hold changed. A transient tool
/// whose last scope left is the idle clock's: stopped `idle_seconds`
/// after its last use, unless something uses it meanwhile.
pub async fn disconnect(daemon: &Daemon, run: &ToolRun, connection: ConnectionId) {
    {
        let mut connections = run.connections.lock().await;
        connections.remove(&connection);
        held(run, &connections);
    }
    if run.service {
        services::held(daemon, run.id).await;
    }
}

/// The word that a connect scope holds the run, or none does, kept in
/// step with `connections` under its lock.
fn held(run: &ToolRun, connections: &HashSet<ConnectionId>) {
    run.held.send_replace(!connections.is_empty());
}
