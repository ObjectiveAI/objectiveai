//! An agent's attachments settled after its start.

use std::sync::Arc;

use super::held;
use crate::daemon::{Daemon, Kind};
use crate::store::{AgentId, ServiceId, services};

/// The agent's container is up and its registration is over: the
/// agent is detached from every service but those in `kept`, the
/// services its registration declared — none, for a program that
/// declared nothing, whose registration asked the daemon for no
/// deploy at all — and each left is told its attachments changed. A
/// store that cannot be written leaves the attachments as they were.
pub async fn settle(daemon: &Arc<Daemon>, agent: AgentId, kept: &[ServiceId]) {
    let left = {
        let Ok(mut tx) = daemon.store.begin().await else {
            return;
        };
        let Ok(left) = services::attachments::detach_except(&mut tx, agent, kept).await else {
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
    daemon.live.changed(Kind::Tools);
}
