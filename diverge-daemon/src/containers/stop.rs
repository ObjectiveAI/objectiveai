//! Ending a run, and what is done when one has ended.

use std::sync::Arc;

use chrono::Utc;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as log, Item, Provider};

use super::{AgentRun, Key, ToolRun, pump, tools};
use crate::daemon::{Daemon, Kind};
use crate::store::{AgentId, agents, tools as tool_records};

/// Stop the agent's run, if one is up: the container is told to stop,
/// and the pump hears the end.
pub async fn stop_agent(daemon: &Daemon, id: AgentId) {
    if let Some(run) = daemon.live.agent_run(id).await {
        let _ = run.handle.stop().await;
    }
}

/// The agent's run has ended, however it did: forgotten as live, its
/// tasks ended, its served tools released, its mounts let go, and the
/// log told it ceased on its provider. Done once: a second call for
/// one run does nothing.
pub async fn ended_agent(daemon: &Daemon, run: Arc<AgentRun>) {
    if daemon.live.remove_agent(run.id).await.is_none() {
        return;
    }
    run.loop_active.send_replace(false);
    for task in run.tasks.lock().await.drain(..) {
        task.abort();
    }
    let ids = run.served.lock().await.ids();
    for id in ids {
        tools::release(daemon, id, Key::Agent(run.id)).await;
    }
    run.mounts.stop().await;
    let _ = pump::append(
        daemon,
        &run,
        Item::Inactive(log::Inactive {
            r#type: Default::default(),
            provider: Provider {
                identity: run.provider.clone(),
            },
        }),
    )
    .await;
    if let Ok(mut conn) = daemon.store.acquire().await {
        let _ = agents::set_last(&mut conn, run.id, &run.provider, Utc::now()).await;
    }
    daemon.live.changed(Kind::Agents);
}

/// The tool's run has ended: forgotten as live, its tasks ended, its
/// own served dependencies released, its mounts let go.
pub async fn ended_tool(daemon: &Daemon, run: Arc<ToolRun>) {
    if daemon.live.remove_tool(run.id).await.is_none() {
        return;
    }
    for task in run.tasks.lock().await.drain(..) {
        task.abort();
    }
    let ids = run.served.lock().await.ids();
    for id in ids {
        tools::release(daemon, id, Key::Tool(run.id)).await;
    }
    run.mounts.stop().await;
    if let Ok(mut conn) = daemon.store.acquire().await {
        let _ = tool_records::set_last(&mut conn, run.id, &run.provider, Utc::now()).await;
    }
}

/// The daemon stops: every run told to stop and taken down now,
/// without waiting on the provider.
pub async fn stop_all(daemon: &Daemon) {
    for run in daemon.live.agent_runs().await {
        let _ = run.handle.stop().await;
        ended_agent(daemon, run).await;
    }
    for run in daemon.live.tool_runs().await {
        run.handle.stop().await;
        ended_tool(daemon, run).await;
    }
}
