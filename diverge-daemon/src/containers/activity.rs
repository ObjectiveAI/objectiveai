//! Whether an agent is active, followed: its log, its `last_active`
//! and its lists told at every turn.

use std::sync::Arc;

use super::AgentRun;
use super::lifecycle;
use crate::daemon::{Daemon, Kind};
use crate::store::AgentId;

/// Follow the agent's activity until the run ends. The rules:
///
/// 1. **Active is two things.** The agent is active exactly while a
///    loop runs in it or an MCP exchange is in flight for it — its
///    container's own, a dependency's, or one run as it through
///    `agents connect` — as [`AgentRun::is_active`] states.
/// 2. **Every turn is kept.** When it becomes active, and when it
///    ceases, the log keeps `active` or `inactive` — only while this
///    run is the one deployed, as
///    [`lifecycle::active`](super::lifecycle::active) gates it — now is
///    kept as its `last_active`, and every open list of agents is told.
/// 3. **Only turns.** A loop beginning while an exchange is in flight,
///    or an exchange beginning in a loop, is no turn, and nothing is
///    kept or told for it.
/// 4. **A time that cannot be kept fails nothing.**
///
/// The run starts inactive, and is spawned after its `deployed`; its
/// end, however it comes, is told by the stop, which keeps the last
/// `inactive` itself before `undeployed`. A run whose exchange was cut
/// short is held active by its log until [`cut`]'s probe proves it
/// alive.
pub async fn activity(daemon: Arc<Daemon>, run: Arc<AgentRun>) {
    let mut looping = run.loop_active.subscribe();
    let mut inflight = run.inflight.subscribe();
    let mut was = false;
    loop {
        let now = *looping.borrow_and_update() || *inflight.borrow_and_update() > 0;
        // Rule 2: told at every change, since a cut run's log may hold it
        // active past a turn — the gate keeps only what moves the log.
        lifecycle::active(daemon.logs.clone(), run.id, Arc::clone(&run.log), run.epoch, now).await;
        // Rules 2 and 3.
        if now != was {
            was = now;
            if let Ok(mut conn) = daemon.store.acquire().await {
                let _ = crate::store::agents::set_active(&mut conn, run.id, chrono::Utc::now()).await;
            }
            daemon.live.changed(Kind::Agents);
        }
        tokio::select! {
            changed = looping.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            changed = inflight.changed() => {
                if changed.is_err() {
                    return;
                }
            }
        }
    }
}

/// An exchange for the agent was cut short — a scope or a connection
/// under it gone — which is no clean completion: the agent's log holds
/// it active, as rule 2a of [`lifecycle`] states, and a probe asks the
/// agent's own run scope one question. Answered, the run is alive and
/// the mark is cleared, `inactive` kept if the agent is idle by then;
/// unanswered, the run is going down, and its `undeployed` keeps the
/// `inactive` itself, interrupted. The probe has no deadline: it waits
/// for the answer, or the scope's end.
pub async fn cut(daemon: &Arc<Daemon>, id: AgentId) {
    let Some(run) = daemon.live.agent_run(id).await else {
        return;
    };
    lifecycle::cut(Arc::clone(&run.log), run.epoch).await;
    let root = daemon.logs.clone();
    tokio::spawn(async move {
        if run.handle.info().await.is_ok() {
            let asked = Arc::clone(&run);
            lifecycle::uncut(root, run.id, Arc::clone(&run.log), run.epoch, move || asked.is_active()).await;
        }
    });
}
