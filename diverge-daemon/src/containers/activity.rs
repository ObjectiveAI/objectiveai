//! Whether an agent is active, followed: its `last_active` kept and
//! its lists told at every turn.

use std::sync::Arc;

use super::AgentRun;
use crate::daemon::{Daemon, Kind};

/// Follow the agent's activity until the run ends. The rules:
///
/// 1. **Active is two things.** The agent is active exactly while a
///    loop runs in it or an MCP exchange is in flight for it — its
///    container's own, a dependency's, or one run as it through
///    `agents connect` — as [`AgentRun::is_active`] states.
/// 2. **Every turn is kept.** When it becomes active, and when it
///    ceases, now is kept as its `last_active`, and every open list of
///    agents is told.
/// 3. **Only turns.** A loop beginning while an exchange is in flight,
///    or an exchange beginning in a loop, is no turn, and nothing is
///    kept or told for it.
/// 4. **A time that cannot be kept fails nothing.**
///
/// The run starts inactive; its end, however it comes, is told by the
/// stop.
pub async fn activity(daemon: Arc<Daemon>, run: Arc<AgentRun>) {
    let mut looping = run.loop_active.subscribe();
    let mut inflight = run.inflight.subscribe();
    let mut was = false;
    loop {
        let now = *looping.borrow_and_update() || *inflight.borrow_and_update() > 0;
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
