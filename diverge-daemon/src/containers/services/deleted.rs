//! What a delete does to the services it reaches.

use std::sync::Arc;

use super::{held, retire};
use crate::containers::{Key, ToolKey};
use crate::daemon::Daemon;
use crate::store::dependency_tools::DependencyTool;
use crate::store::{DependencyToolId, ToolId};

/// The agent is deleted: `removed` were the dependency tools it
/// owned, whose rows went with it — each service among them retired,
/// its run with it — `left` the dependency tools it was attached to
/// and did not own, and `detached` the tools on record it was
/// attached to, each told its hold changed, which starts a service's
/// clock where the agent was the last to hold it.
pub async fn agent_deleted(daemon: &Arc<Daemon>, removed: Vec<DependencyTool>, left: Vec<DependencyToolId>, detached: Vec<ToolId>) {
    template_deleted(daemon, removed).await;
    for id in left {
        held(daemon, ToolKey::Dependency(id)).await;
    }
    for id in detached {
        held(daemon, ToolKey::Record(id)).await;
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
            retire(daemon, ToolKey::Dependency(tool.id)).await;
        } else {
            daemon.live.forget_scope(Key::Tool(ToolKey::Dependency(tool.id))).await;
        }
    }
}

/// The tool on record is deleted: its keeping, if it was a service
/// kept — by a connect scope, or pending its clock, since a delete is
/// refused while any agent is attached — is ended, its run with it.
pub async fn tool_deleted(daemon: &Arc<Daemon>, id: ToolId) {
    retire(daemon, ToolKey::Record(id)).await;
}
