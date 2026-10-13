//! Ending a run, and what is done when one has ended.

use std::sync::Arc;

use chrono::Utc;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as log, Item, Provider};
use futures_util::future;

use super::{AgentRun, Key, ToolKey, ToolRun, User, pump, services, tools};
use crate::daemon::{Daemon, Kind};
use crate::store::{AgentId, agents, services as service_records, tools as tool_records};

/// Stop the agent's run, if one is up: the container is told to stop,
/// and the pump hears the end.
pub async fn stop_agent(daemon: &Daemon, id: AgentId) {
    if let Some(run) = daemon.live.agent_run(id).await {
        let _ = run.handle.stop().await;
    }
}

/// The agent's run has ended, however it did: forgotten as live, its
/// tasks ended, its transient dependencies stopped and its attached
/// tools released — all at once, every container told at the same
/// time; its services left to their attachments —
/// its mounts let go, and the log told it ceased on its provider.
/// Done once: a second call for one run does nothing.
pub async fn ended_agent(daemon: &Daemon, run: Arc<AgentRun>) {
    if daemon.live.remove_agent(run.id).await.is_none() {
        return;
    }
    run.loop_active.send_replace(false);
    for task in run.tasks.lock().await.drain(..) {
        task.abort();
    }
    let (dependencies, attached) = {
        let served = run.served.lock().await;
        (served.dependencies(), served.attached_running())
    };
    future::join(
        stop_dependencies(daemon, dependencies),
        future::join_all(attached.into_iter().map(|key| tools::release(daemon, key, User::Container(Key::Agent(run.id))))),
    )
    .await;
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

/// Every transient dependency stopped at once: each told to stop,
/// each waited for, each taken down.
pub async fn stop_dependencies(daemon: &Daemon, dependencies: Vec<Arc<ToolRun>>) {
    future::join_all(dependencies.iter().map(|run| async move {
        run.handle.stop().await;
        run.handle.wait().await;
    }))
    .await;
    for run in dependencies {
        ended_tool(daemon, run).await;
    }
}

/// The tool's run has ended: forgotten as live, its tasks ended, the
/// word that it ended said — which finishes every connect scope on it
/// — its mounts let go; a record's told where and when it last ran; a
/// transient dependency's taken out of its agent's served set, which
/// tells the agent its tools changed, and its scope's password
/// forgotten, since its key is never seen again; a service's told
/// where it last ran, its scope and its place in every served set
/// kept for the next run, which its supervisor starts while any agent
/// is attached.
pub async fn ended_tool(daemon: &Daemon, run: Arc<ToolRun>) {
    if daemon.live.remove_tool(run.id).await.is_none() {
        return;
    }
    for task in run.tasks.lock().await.drain(..) {
        task.abort();
    }
    run.ended.send_replace(true);
    run.mounts.stop().await;
    match (run.id, &run.dependency) {
        (ToolKey::Record(id), _) => {
            if let Ok(mut conn) = daemon.store.acquire().await {
                let _ = tool_records::set_last(&mut conn, id, &run.provider, Utc::now()).await;
            }
        }
        (ToolKey::Dependency(_), Some(dependency)) => {
            if let Some(agent) = daemon.live.agent_run(dependency.agent).await {
                agent.served.lock().await.remove(run.id);
            }
            daemon.live.forget_scope(Key::Tool(run.id)).await;
        }
        (ToolKey::Dependency(_), None) => {
            daemon.live.forget_scope(Key::Tool(run.id)).await;
        }
        (ToolKey::Service(id), _) => {
            if let Ok(mut conn) = daemon.store.acquire().await {
                let _ = service_records::set_last(&mut conn, id, &run.provider).await;
            }
        }
    }
    daemon.live.changed(Kind::StandaloneTools);
}

/// The daemon stops: every service's supervisor stilled first, so
/// that no run comes back, then every run told to stop and taken
/// down now, without waiting on the provider.
pub async fn stop_all(daemon: &Daemon) {
    services::stop_all(daemon).await;
    for run in daemon.live.agent_runs().await {
        let _ = run.handle.stop().await;
        ended_agent(daemon, run).await;
    }
    for run in daemon.live.tool_runs().await {
        run.handle.stop().await;
        ended_tool(daemon, run).await;
    }
}
