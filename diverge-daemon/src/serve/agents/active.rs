//! What an agent is doing now.

use crate::containers::AgentRun;
use crate::daemon::Daemon;
use crate::judge::AgentState;
use crate::store::AgentId;

/// The agent's state now, as [`AgentState`] states it: a loop running,
/// its container up, and either or an exchange in flight. What a list
/// reports, what a filter and a grant's `within` may ask about, and —
/// `busy` — what refuses a delete.
pub async fn state(daemon: &Daemon, id: AgentId) -> AgentState {
    state_of(daemon.live.agent_run(id).await.as_deref())
}

/// The state a run makes: none, nothing.
pub fn state_of(run: Option<&AgentRun>) -> AgentState {
    match run {
        Some(run) => AgentState {
            active: *run.loop_active.borrow(),
            deployed: true,
            busy: run.is_active(),
        },
        None => AgentState::default(),
    }
}
