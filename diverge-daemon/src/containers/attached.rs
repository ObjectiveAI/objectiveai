//! A tool attached or detached while agents run: their served sets
//! kept in step, and each told its tool list changed.

use std::sync::Arc;

use super::{ToolKey, serve_name};
use crate::daemon::Daemon;
use crate::store::tools::Tool;
use crate::store::{AgentId, ToolId};

/// The tool is attached to every one of `agents` now — directly, or
/// through their template: each one running has it served from this
/// instant, under its name, and hears its tool list changed; one
/// served it already keeps it as it was. The tool's container is
/// started at the agent's next use of it, not here.
pub async fn attached(daemon: &Arc<Daemon>, tool: &Tool, agents: &[AgentId]) {
    for agent in agents {
        if let Some(run) = daemon.live.agent_run(*agent).await {
            let name = serve_name(tool);
            run.served.lock().await.insert(tool.clone(), &name);
        }
    }
}

/// The tool is detached from every one of `agents` one way — directly,
/// or through their template — and `keeps` are the agents that have it
/// still the other way, by the records as of the detach: each of the
/// rest that is running serves it no more from this instant and hears
/// its tool list changed. An exchange in flight to the tool is
/// answered still, holding the tool's run up until it is; nothing of
/// the run is stopped here.
pub async fn detached(daemon: &Arc<Daemon>, tool: ToolId, agents: &[AgentId], keeps: &[AgentId]) {
    for agent in agents {
        if keeps.contains(agent) {
            continue;
        }
        if let Some(run) = daemon.live.agent_run(*agent).await {
            run.served.lock().await.remove(ToolKey::Record(tool));
        }
    }
}
