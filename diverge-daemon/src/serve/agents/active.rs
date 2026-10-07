//! Whether a loop runs in an agent.

use crate::daemon::Daemon;
use crate::store::AgentId;

/// Whether a loop runs in the agent now — its container up and the
/// proxy's last word `active`: what a list reports as `active`, what
/// refuses a delete, a detach and an edit of the mounts, and what a
/// filter and a grant's `within` may ask about.
pub async fn active(daemon: &Daemon, id: AgentId) -> bool {
    daemon.live.agent_run(id).await.is_some_and(|run| run.is_active())
}
