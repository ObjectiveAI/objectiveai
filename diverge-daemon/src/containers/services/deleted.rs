//! What a delete does to the services it reaches.

use std::sync::Arc;

use super::{held, retire};
use crate::daemon::Daemon;
use crate::store::services::Service;
use crate::store::{AgentId, ServiceId};

/// The agent is deleted, and `services` were the ones it was attached
/// to: each per-agent service of its own is retired — its row went
/// with the agent's, and its run goes here — and every other is told
/// its attachments changed, which starts its clock where the agent
/// was the last attached.
pub async fn agent_deleted(daemon: &Arc<Daemon>, agent: AgentId, services: Vec<Service>) {
    for service in services {
        if service.owner_agent == Some(agent) {
            retire(daemon, service.id).await;
        } else {
            held(daemon, service.id).await;
        }
    }
}

/// The agent template is deleted, and `removed` were its per-template
/// services, whose rows went with it: each is retired, its run with
/// it.
pub async fn template_deleted(daemon: &Arc<Daemon>, removed: Vec<ServiceId>) {
    for id in removed {
        retire(daemon, id).await;
    }
}
