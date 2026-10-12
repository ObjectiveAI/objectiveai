//! What a service is attached to, and whose it is, loaded.

use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use crate::store::services::{self, Service};
use crate::store::{self, agents};

/// The agents attached to the service, in attach order, each as its
/// key — its name as it is called now.
pub async fn service_agents(conn: &mut PgConnection, service: &Service) -> Result<Vec<key::Agent>, store::Error> {
    let mut attached = Vec::new();
    for id in services::attachments::attached(conn, service.id).await? {
        if let Some(agent) = agents::by_id(conn, id, false).await? {
            attached.push(agent.key());
        }
    }
    Ok(attached)
}

/// Whose the service is, as the wire names it: its owner agent loaded
/// for a per-agent service. `None` for a per-agent service whose
/// agent is gone, which is a service on its way out.
pub async fn service_owner(conn: &mut PgConnection, service: &Service) -> Result<Option<key::Owner>, store::Error> {
    let owner = match service.owner_agent {
        Some(id) => agents::by_id(conn, id, false).await?,
        None => None,
    };
    Ok(service.owner(owner.as_ref()))
}
