//! One container per tool on record: started for the first thing that
//! uses it, held up by what is in flight into it and the connect
//! scopes on it, and — a transient tool — stopped by its idle clock
//! `idle_seconds` after the last of those left; a service is kept
//! while anything is attached to it and retired by its own clock.

use std::collections::HashSet;
use std::sync::Arc;

use super::{ConnectionId, StartError, ToolKey, ToolRun, services, start};
use crate::daemon::Daemon;
use crate::store::connected_standalone_tools::ConnectedTool;
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

/// The connected tool's run — this daemon's connect on the other
/// daemon's tool: opened now if it was not held, and touched if it was.
/// Nothing is held by this, as for [`use_tool`]; a connected tool is
/// never a service here, and never served onward to another daemon.
pub async fn use_connected(daemon: &Arc<Daemon>, tool: &ConnectedTool) -> Result<Arc<ToolRun>, StartError> {
    match daemon.live.tool_run(ToolKey::Connected(tool.id)).await {
        Some(run) => {
            run.touch();
            Ok(run)
        }
        None => start::connected(daemon, tool).await,
    }
}

/// A connect scope of another daemon's holds the run from now on,
/// until the [`Hold`] handed back is let go — [`Hold::disconnect`] on
/// the scope's end, or the guard dropped however else it ends: the run
/// is told it is held.
pub async fn connect(daemon: &Arc<Daemon>, run: &Arc<ToolRun>, connection: ConnectionId) -> Hold {
    let mut connections = run.connections.lock().await;
    connections.insert(connection);
    held(run, &connections);
    Hold { daemon: Arc::clone(daemon), run: Arc::clone(run), connection: Some(connection) }
}

/// One connect scope's hold on a run, from [`connect`]. A drop cannot
/// wait on the run's lock, so a hold dropped [`disconnect`]s in a task
/// of its own.
pub struct Hold {
    /// The daemon.
    daemon: Arc<Daemon>,
    /// The run held.
    run: Arc<ToolRun>,
    /// The scope's number, until it is disconnected.
    connection: Option<ConnectionId>,
}

impl Hold {
    /// The connect scope is over: [`disconnect`], waited for.
    pub async fn disconnect(mut self) {
        if let Some(connection) = self.connection {
            disconnect(&self.daemon, &self.run, connection).await;
            self.connection = None;
        }
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        let Some(connection) = self.connection.take() else {
            return;
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let (daemon, run) = (Arc::clone(&self.daemon), Arc::clone(&self.run));
            runtime.spawn(async move { disconnect(&daemon, &run, connection).await });
        }
    }
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
