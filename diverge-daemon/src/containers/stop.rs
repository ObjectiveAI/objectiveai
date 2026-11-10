//! Ending a run, and what is done when one has ended.

use std::sync::Arc;

use chrono::Utc;
use diverge_sdk::daemon::lifecycle::Reason;
use futures_util::future;

use super::lifecycle::{self, End};
use super::{AgentRun, Key, ToolHandle, ToolKey, ToolRun, secrets, services, tool_log};
use crate::daemon::{Daemon, Kind};
use crate::store::{AgentId, agents, connected_standalone_tools as connected_records, tools as tool_records};

/// Stop the agent's run, if one is up, as asked: the container is told
/// to stop, and the pump hears the end.
pub async fn stop_agent(daemon: &Daemon, id: AgentId) {
    if let Some(run) = daemon.live.agent_run(id).await {
        run.end_as(End::Expected).await;
        let _ = run.handle.cancel().await;
    }
}

/// End the agent's run as failed, for a batch at its proxy that could
/// not be fed: the gate marked `ending` under its lock, so nothing
/// more is activated on the run or sent to it and the messages after
/// wait for the next — see [`activate_agent`](super::activate_agent)
/// — and the container told to stop, which takes the batch with it.
/// The pump hears the end.
pub async fn fail_agent(run: &AgentRun) {
    run.gate.lock().await.ending = true;
    run.end_as(End::Unexpected(Reason::Failed)).await;
    let _ = run.handle.cancel().await;
}

/// [`ended_agent`] in a task of its own. The end of a run is never
/// taken in one of the run's own tasks — the pump, a waiter — since
/// the end aborts every one of them: so the pump, hearing the end,
/// hands it here and returns.
pub fn end_agent(daemon: Arc<Daemon>, run: Arc<AgentRun>) {
    tokio::spawn(async move { ended_agent(&daemon, run).await });
}

/// [`ended_tool`] in a task of its own, as [`end_agent`].
pub fn end_tool(daemon: Arc<Daemon>, run: Arc<ToolRun>) {
    tokio::spawn(async move { ended_tool(&daemon, run).await });
}

/// The agent's run has ended, however it did: forgotten as live, its
/// tasks ended — never the task this runs in: see [`end_agent`] — the
/// log told it went down, with the reason the run's [`End`] gives and
/// whether it was working, before anything else; the vault slots it
/// held released with their values unchanged, its transient
/// dependencies stopped as the agent ended — all at once, every
/// container told at the same time; its attached tools left to their
/// own use and their idle clocks, its services to their attachments
/// — its mounts let go, and the provider's capacity asked again,
/// since the run's reservation is freed. Done once, and only for the
/// run live under the agent: a second call for one run, or a late one
/// for a run a newer one replaced, does nothing.
pub async fn ended_agent(daemon: &Daemon, run: Arc<AgentRun>) {
    if !daemon.live.remove_agent_if(&run).await {
        return;
    }
    down_agent(daemon, run).await;
}

/// A start that failed once its agent's run was made — its container
/// open, its secrets moved in, perhaps its `deployed` kept — undone as
/// any run's end takes it down: the container told to stop, and the
/// run taken down as [`ended_agent`] takes one, `live` saying whether
/// it was entered as live yet. The one undo every such failure goes
/// through.
pub async fn undo_agent(daemon: &Daemon, run: Arc<AgentRun>, live: bool) {
    run.end_as(End::Expected).await;
    let _ = run.handle.cancel().await;
    if live {
        ended_agent(daemon, run).await;
    } else {
        down_agent(daemon, run).await;
    }
}

/// What [`ended_agent`] does once the run is no longer live, or never
/// was; and last, the word that the run is gone, which the agent's
/// outbox pump waits for before it starts the next — see
/// [`AgentRun::gone`] — so a run is started only once the one before
/// it is down whole.
async fn down_agent(daemon: &Daemon, run: Arc<AgentRun>) {
    for task in run.tasks.lock().await.drain(..) {
        task.abort();
    }
    let end = run.end().await;
    lifecycle::undeployed(daemon.logs.clone(), run.id, Arc::clone(&run.log), run.epoch, end.reason()).await;
    run.loop_active.send_replace(false);
    if let Some(mut held) = run.gate.lock().await.secrets.take() {
        let _ = secrets::release(daemon, &mut held, &run.sender).await;
    }
    let dependencies = run.served.lock().await.transients();
    stop_dependencies(daemon, dependencies, end).await;
    run.mounts.stop().await;
    if let Ok(mut conn) = daemon.store.acquire().await {
        let _ = agents::set_deployed(&mut conn, run.id, &run.provider, Utc::now()).await;
    }
    daemon.live.changed(Kind::Agents);
    daemon.live.nudge_capacity(&run.provider);
    run.ended.send_replace(true);
}

/// Every transient dependency tool stopped at once, ending as `end`
/// says — its agent's end, or a deploy's undoing: each told to stop,
/// each waited for, each taken down.
pub async fn stop_dependencies(daemon: &Daemon, dependencies: Vec<Arc<ToolRun>>, end: End) {
    future::join_all(dependencies.iter().map(|run| async move {
        run.end_as(end).await;
        run.handle.stop().await;
        run.handle.wait().await;
    }))
    .await;
    for run in dependencies {
        ended_tool(daemon, run).await;
    }
}

/// The tool's run has ended: forgotten as live, its tasks ended —
/// never the task this runs in: see [`end_tool`] — the log told it
/// went down, with the reason the run's [`End`] gives, the open asks
/// answered interrupted first, and the tool marked not working when it
/// was; the vault slots it held released with their values unchanged, the
/// word that it ended said — which finishes every connect scope on it
/// — its mounts let go; a record's told where and when it last ran; a
/// transient dependency tool's taken out of its agent's served set,
/// which tells the agent its tools changed, and its scope's password
/// forgotten, since its row is next run from a new agent run; a
/// service's scope and its place in every served set kept for the
/// next run, which its supervisor starts while any agent is attached.
/// Every session's notifications reader is ended, and the tool marked
/// working no more, whatever its log said. Done once, and only for the
/// run live under its key, as [`ended_agent`].
pub async fn ended_tool(daemon: &Daemon, run: Arc<ToolRun>) {
    if !daemon.live.remove_tool_if(&run).await {
        return;
    }
    down_tool(daemon, run).await;
}

/// A start that failed once its tool's run was made, undone as
/// [`undo_agent`] undoes an agent's: the container told to stop, and
/// the run taken down as [`ended_tool`] takes one.
pub async fn undo_tool(daemon: &Daemon, run: Arc<ToolRun>, live: bool) {
    run.end_as(End::Expected).await;
    run.handle.stop().await;
    if live {
        ended_tool(daemon, run).await;
    } else {
        down_tool(daemon, run).await;
    }
}

/// What [`ended_tool`] does once the run is no longer live, or never
/// was.
async fn down_tool(daemon: &Daemon, run: Arc<ToolRun>) {
    daemon.live.set_busy(run.id, false).await;
    for task in run.tasks.lock().await.drain(..) {
        task.abort();
    }
    tool_log::end_sessions(&run).await;
    let end = run.end().await;
    if lifecycle::undeployed(daemon.logs.clone(), run.id, Arc::clone(&run.log), run.epoch, end.reason()).await {
        tool_log::working(daemon, run.id, false).await;
    }
    if let Some(mut held) = run.secrets.lock().await.take() {
        let _ = secrets::release(daemon, &mut held, &run.sender).await;
    }
    run.ended.send_replace(true);
    run.mounts.stop().await;
    if let ToolKey::Dependency(id) = run.id
        && let Ok(mut conn) = daemon.store.acquire().await
    {
        let _ = crate::store::dependency_tools::set_deployed(&mut conn, id, Utc::now()).await;
    }
    match (run.id, &run.dependency) {
        (ToolKey::Record(id), _) => {
            if let Ok(mut conn) = daemon.store.acquire().await {
                let _ = tool_records::set_deployed(&mut conn, id, &run.provider, Utc::now()).await;
            }
        }
        (ToolKey::Connected(id), _) => {
            if let Ok(mut conn) = daemon.store.acquire().await {
                let _ = connected_records::set_deployed(&mut conn, id, Utc::now()).await;
            }
        }
        (ToolKey::Dependency(_), Some(tool)) if !tool.is_service() => {
            if let Some(agent) = tool.owner_agent {
                if let Some(agent) = daemon.live.agent_run(agent).await {
                    agent.served.lock().await.remove_transient(&run);
                }
            }
            daemon.live.forget_scope(Key::Tool(run.id)).await;
        }
        (ToolKey::Dependency(_), _) => {}
    }
    daemon.live.changed(Kind::DependencyTools);
    daemon.live.changed(Kind::StandaloneTools);
    daemon.live.changed(Kind::ConnectedStandaloneTools);
    // A connected tool ran on another daemon's provider: nothing of
    // this daemon's was reserved there, and nothing is freed.
    if let ToolHandle::Run(_) = run.handle {
        daemon.live.nudge_capacity(&run.provider);
    }
}

/// The daemon stops: every service's supervisor stilled first, so
/// that no run comes back, then every run told to stop and taken
/// down now, without waiting on the provider.
pub async fn stop_all(daemon: &Daemon) {
    services::stop_all(daemon).await;
    for run in daemon.live.agent_runs().await {
        run.end_as(End::Unexpected(Reason::DaemonStopped)).await;
        let _ = run.handle.cancel().await;
        ended_agent(daemon, run).await;
    }
    for run in daemon.live.tool_runs().await {
        run.end_as(End::Unexpected(Reason::DaemonStopped)).await;
        run.handle.stop().await;
        ended_tool(daemon, run).await;
    }
}
