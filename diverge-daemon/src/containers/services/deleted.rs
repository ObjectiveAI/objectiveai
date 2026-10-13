//! What a delete does to the services it reaches.

use std::sync::Arc;

use super::{held, retire};
use crate::containers::{Key, ToolKey};
use crate::daemon::Daemon;
use crate::store::DependencyToolId;
use crate::store::dependency_tools::DependencyTool;

/// The agent is deleted: `removed` were the dependency tools it
/// owned, whose rows went with it — each service among them retired,
/// its run with it — and `left` the tools it was attached to and did
/// not own, each told its attachments changed, which starts its clock
/// where the agent was the last attached.
pub async fn agent_deleted(daemon: &Arc<Daemon>, removed: Vec<DependencyTool>, left: Vec<DependencyToolId>) {
    template_deleted(daemon, removed).await;
    for id in left {
        held(daemon, id).await;
    }
}

/// The agent template, or the dependency template, is deleted, and
/// `removed` were the dependency tools whose rows went with it — the
/// agent template's per-template ones, or every one of the dependency
/// template: each service among them is retired, its run with it; a
/// transient tool among them has no keeping to end, and only its
/// scope's password to forget.
pub async fn template_deleted(daemon: &Arc<Daemon>, removed: Vec<DependencyTool>) {
    for tool in removed {
        if tool.is_service() {
            retire(daemon, tool.id).await;
        } else {
            daemon.live.forget_scope(Key::Tool(ToolKey::Dependency(tool.id))).await;
        }
    }
}
