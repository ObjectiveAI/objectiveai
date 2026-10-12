//! One tool as a list reports it.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Provider;
use diverge_sdk::daemon::endpoints::tools::list::server::response;
use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use super::{active, agents_of, running};
use crate::containers::ToolRun;
use crate::daemon::Daemon;
use crate::store::services::Service;
use crate::store::tools::Tool;
use crate::store;

/// The record as a list reports it: its attachments loaded now, and
/// whether it is active now.
pub async fn report(conn: &mut PgConnection, daemon: &Daemon, tool: &Tool) -> Result<response::Tool, store::Error> {
    let agents = agents_of(conn, tool.id).await?;
    Ok(tool.report(active(daemon, tool.id).await, running(daemon, tool.id).await, agents))
}

/// The transient dependency as a list reports it, from its run alone:
/// no name, active, attached to the one agent it was deployed for,
/// made by itself when it was deployed, with no tags. `None` for a
/// run that is a record's or a service's.
pub fn report_dependency(run: &ToolRun) -> Option<response::Tool> {
    let dependency = run.dependency.as_ref()?;
    Some(response::Tool {
        name: None,
        origin: response::Origin::Dependency {
            agent: dependency.agent_key.clone(),
            template: dependency.template.clone(),
            declared: dependency.declared.clone(),
            provider: Provider {
                identity: run.provider.clone(),
            },
            id: run.container.clone().unwrap_or_default(),
        },
        creator: run.sender.clone(),
        created: dependency.started,
        active: true,
        last_active: Some(dependency.started),
        agents: vec![dependency.agent_key.clone()],
        tags: Vec::new(),
    })
}

/// The service as a list reports it: no name, whose it is, the agents
/// attached to it, made by the agent whose registration made it, with
/// no tags; active while its container runs, with the container's id
/// then, on the provider it runs or last ran on.
pub fn report_service(service: &Service, owner: key::Owner, attached: Vec<key::Agent>, active: bool, running: Option<String>) -> response::Tool {
    response::Tool {
        name: None,
        origin: response::Origin::Service {
            template: service.template.clone(),
            owner,
            declared: service.declared.clone(),
            attached: attached.clone(),
            provider: service.last_provider.clone().map(|identity| Provider { identity }),
            id: running,
        },
        creator: service.creator.clone(),
        created: service.created,
        active,
        last_active: None,
        agents: attached,
        tags: Vec::new(),
    }
}
