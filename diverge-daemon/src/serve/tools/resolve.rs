//! The tool a request names: a record, a transient dependency that
//! runs, or a service.

use std::sync::Arc;

use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use super::{active, agents_of, service_agents};
use crate::containers::{ToolKey, ToolRun};
use crate::daemon::Daemon;
use crate::judge::filter::tools::Facts;
use crate::judge::{self, Standing};
use crate::store::services::Service;
use crate::store::tools::Tool;
use crate::store::{self, agents, services, tools};

/// What a reference named: a tool on record; a transient dependency
/// deployed for a running agent, which is no record and is found only
/// while it runs; or a service the agent named is attached to, with
/// its run if one is up.
pub enum Found {
    /// A record.
    Record(Tool),
    /// A transient dependency's run.
    Dependency(Arc<ToolRun>),
    /// A service, and its run while one is up.
    Service {
        /// The row.
        service: Service,
        /// The run, if the service's container runs now.
        run: Option<Arc<ToolRun>>,
    },
}

/// The one sentence every request that would change a dependency
/// answers with: it is read, never changed, and goes with its agents.
pub const READ_ONLY: &str = "a dependency tool is its agents': it is read, not changed";

/// The tool the reference names, if any: by name, by template and
/// index, or by daemon and tool, a record, locked for the
/// transaction when `lock`; by agent and template, the transient
/// dependency deployed from that template for that agent, if the
/// agent runs and serves one, else the service made from that
/// template the agent is attached to, if any.
pub async fn resolve(conn: &mut PgConnection, daemon: &Daemon, reference: &reference::Tool, lock: bool) -> Result<Option<Found>, store::Error> {
    if let reference::Tool::Dependency { agent, template } = reference {
        let Some(agent) = agents::by_reference(conn, agent, false).await? else {
            return Ok(None);
        };
        if let Some(run) = daemon.live.agent_run(agent.id).await {
            if let Some(found) = run.served.lock().await.dependency_of_template(template) {
                return Ok(Some(Found::Dependency(found)));
            }
        }
        let Some(service) = services::of_agent(conn, agent.id)
            .await?
            .into_iter()
            .find(|service| &service.template == template)
        else {
            return Ok(None);
        };
        let run = daemon.live.tool_run(ToolKey::Service(service.id)).await;
        return Ok(Some(Found::Service { service, run }));
    }
    Ok(tools::by_reference(conn, reference, lock).await?.map(Found::Record))
}

/// Whether the standing may do `action` to what was found: a record
/// by its row, whether it is active now and what it is attached to,
/// loaded here; a transient dependency by its run; a service by its
/// row, whether it runs now and what is attached to it, loaded here.
pub async fn reaches(conn: &mut PgConnection, daemon: &Daemon, standing: &Standing, action: Over, found: &Found) -> Result<bool, store::Error> {
    Ok(match found {
        Found::Record(tool) => {
            let attached = agents_of(conn, tool.id).await?;
            judge::tools::over(standing, action, &Facts::record(tool, active(daemon, tool.id).await, &attached))
        }
        Found::Dependency(run) => Facts::dependency(run).is_some_and(|facts| judge::tools::over(standing, action, &facts)),
        Found::Service { service, run } => {
            let attached = service_agents(conn, service).await?;
            judge::tools::over(standing, action, &Facts::service(service, run.is_some(), &attached))
        }
    })
}
