//! A registration's services: found or made, the agent attached to
//! each, detached from the rest.

use std::sync::Arc;

use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::key;
use diverge_sdk::shared::containers::dependencies::{Scope, Template};

use super::{held, keep};
use crate::daemon::{Daemon, Kind};
use crate::store::{AgentId, ServiceId, services};

/// One service a registration declared: its template by id and
/// whole, and its scope.
#[derive(Debug, Clone)]
pub struct Declared {
    /// The template, by id.
    pub template: String,
    /// The template, whole.
    pub declared: Template,
    /// How far it is shared.
    pub scope: Scope,
}

/// The agent's registration declared these services: in one
/// transaction, each found under its scope key or made now — with the
/// agent as its creator — the agent attached to each, and the agent
/// detached from every service it was attached to and did not declare
/// this time; then each declared kept up and its hold recomputed, and
/// each left told its attachments changed. The services, in the
/// order declared, for the agent to be served.
pub async fn register(daemon: &Arc<Daemon>, agent: AgentId, agent_key: &key::Agent, declared: &[Declared]) -> Result<Vec<ServiceId>, String> {
    let creator = Creator::Agent(creator::Agent {
        template: agent_key.template.clone(),
        index: agent_key.index,
        name: agent_key.name.clone(),
    });
    let mut tx = daemon.store.begin().await.map_err(|error| error.to_string())?;
    let mut ids = Vec::with_capacity(declared.len());
    for one in declared {
        let service = services::find_or_create(
            &mut tx,
            &services::Declared {
                template: &one.template,
                declared: &one.declared,
                scope: one.scope,
                agent,
                agent_template: &agent_key.template,
                creator: &creator,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
        services::attachments::attach(&mut tx, service.id, agent)
            .await
            .map_err(|error| error.to_string())?;
        ids.push(service.id);
    }
    let left = services::attachments::detach_except(&mut tx, agent, &ids)
        .await
        .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;
    for id in &ids {
        keep(daemon, *id).await;
        held(daemon, *id).await;
    }
    for id in left {
        held(daemon, id).await;
    }
    daemon.live.changed(Kind::StandaloneTools);
    Ok(ids)
}
