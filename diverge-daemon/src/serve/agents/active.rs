//! Whether a loop runs in an agent.

use crate::daemon::Daemon;
use crate::store::AgentId;

/// Whether a loop runs in the agent now: what a list reports as
/// `active`, what refuses a delete, a detach and an edit of the
/// mounts, and what a filter and a grant's `within` may ask about. No
/// container runs yet.
pub fn active(_: &Daemon, _: AgentId) -> bool {
    false
}
