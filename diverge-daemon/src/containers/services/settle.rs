//! An agent's dependencies settled after a start whose program
//! declared none.

use std::sync::Arc;

use super::held;
use crate::daemon::{Daemon, Kind};
use crate::store::{AgentId, dependencies, services};

/// The agent's container is up and its registration asked the daemon
/// for no deploy at all: the agent depends on nothing — its
/// dependencies reset to none, as a registration that declared none
/// would — and is detached from every service, each left told its
/// attachments changed. A store that cannot be written leaves the
/// records as they were.
pub async fn settle(daemon: &Arc<Daemon>, agent: AgentId) {
    let left = {
        let Ok(mut tx) = daemon.store.begin().await else {
            return;
        };
        let Ok(left) = services::attachments::detach_except(&mut tx, agent, &[]).await else {
            return;
        };
        if dependencies::agents::replace(&mut tx, agent, &[]).await.is_err() {
            return;
        }
        if tx.commit().await.is_err() {
            return;
        }
        left
    };
    daemon.live.changed(Kind::DependencyTools);
    if left.is_empty() {
        return;
    }
    for id in left {
        held(daemon, id).await;
    }
    daemon.live.changed(Kind::StandaloneTools);
}
