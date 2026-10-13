//! An agent's dependencies settled after a start whose program
//! declared none.

use std::sync::Arc;

use super::held;
use crate::daemon::{Daemon, Kind};
use crate::store::{AgentId, dependency_tools};

/// The agent's container is up and its registration asked the daemon
/// for no deploy at all: the agent depends on nothing — detached from
/// every dependency tool, as a registration that declared none would
/// detach it — and each left is told its attachments changed. A store
/// that cannot be written leaves the attachments as they were.
pub async fn settle(daemon: &Arc<Daemon>, agent: AgentId) {
    let left = {
        let Ok(mut tx) = daemon.store.begin().await else {
            return;
        };
        let Ok(left) = dependency_tools::attachments::detach_except(&mut tx, agent, &[]).await else {
            return;
        };
        if tx.commit().await.is_err() {
            return;
        }
        left
    };
    if left.is_empty() {
        return;
    }
    for id in left {
        held(daemon, id).await;
    }
    daemon.live.changed(Kind::DependencyTools);
}
