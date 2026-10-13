//! The dependency tool a reference names, if it runs.

use std::sync::Arc;

use diverge_sdk::daemon::reference;
use diverge_sdk::shared::containers::dependencies::Scope;
use sqlx::PgConnection;

use super::{ToolKey, ToolRun};
use crate::daemon::Daemon;
use crate::store::{self, agents, services};

/// The tool running from the template for the owner, if one does:
/// for an agent, the transient tool deployed from the template for
/// the agent's run, else the per-agent service's run; for an agent
/// template or everyone, the service's run under that scope key. A
/// template that is none, an owner the template has no tool for, and
/// a tool that does not run are each `None`: nothing is started.
pub async fn running_dependency(daemon: &Daemon, conn: &mut PgConnection, reference: &reference::Dependency) -> Result<Option<Arc<ToolRun>>, store::Error> {
    let template = reference.template.as_str();
    let service = match &reference.owner {
        reference::Owner::Agent(agent) => {
            let Some(agent) = agents::by_reference(conn, agent, false).await? else {
                return Ok(None);
            };
            let transient = daemon
                .live
                .tool_runs()
                .await
                .into_iter()
                .find(|run| run.dependency.as_ref().is_some_and(|dependency| dependency.agent == agent.id && dependency.template == template));
            if let Some(run) = transient {
                return Ok(Some(run));
            }
            services::by_scope(conn, template, Scope::Instance, Some(agent.id), None).await?
        }
        reference::Owner::AgentTemplate { template: owner } => services::by_scope(conn, template, Scope::Template, None, Some(owner.as_str())).await?,
        reference::Owner::Global => services::by_scope(conn, template, Scope::Global, None, None).await?,
    };
    Ok(match service {
        Some(service) => daemon.live.tool_run(ToolKey::Service(service.id)).await,
        None => None,
    })
}
