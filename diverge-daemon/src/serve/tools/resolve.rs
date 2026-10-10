//! The tool a request names: a record, or a dependency that runs.

use std::sync::Arc;

use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use super::{active, agents_of};
use crate::containers::ToolRun;
use crate::daemon::Daemon;
use crate::judge::filter::tools::Facts;
use crate::judge::{self, Standing};
use crate::store::tools::Tool;
use crate::store::{self, agents, tools};

/// What a reference named: a tool on record, or a dependency deployed
/// for a running agent, which is no record and is found only while
/// it runs.
pub enum Found {
    /// A record.
    Record(Tool),
    /// A dependency's run.
    Dependency(Arc<ToolRun>),
}

/// The one sentence every request that would change a dependency
/// answers with: it is read, never changed, and goes with its agent.
pub const READ_ONLY: &str = "a dependency tool is its agent's: it is read, not changed";

/// The tool the reference names, if any: by name, by template and
/// index, or by daemon and tool, a record, locked for the
/// transaction when `lock`; by agent and template, the dependency
/// deployed from that template for that agent, if the agent runs and
/// serves one.
pub async fn resolve(conn: &mut PgConnection, daemon: &Daemon, reference: &reference::Tool, lock: bool) -> Result<Option<Found>, store::Error> {
    if let reference::Tool::Dependency { agent, template } = reference {
        let Some(agent) = agents::by_reference(conn, agent, false).await? else {
            return Ok(None);
        };
        let Some(run) = daemon.live.agent_run(agent.id).await else {
            return Ok(None);
        };
        let found = run.served.lock().await.dependency_of_template(template);
        return Ok(found.map(Found::Dependency));
    }
    Ok(tools::by_reference(conn, reference, lock).await?.map(Found::Record))
}

/// Whether the standing may do `action` to what was found: a record
/// by its row, whether it is active now and what it is attached to,
/// loaded here; a dependency by its run.
pub async fn reaches(conn: &mut PgConnection, daemon: &Daemon, standing: &Standing, action: Over, found: &Found) -> Result<bool, store::Error> {
    Ok(match found {
        Found::Record(tool) => {
            let attached = agents_of(conn, tool.id).await?;
            judge::tools::over(standing, action, &Facts::record(tool, active(daemon, tool.id).await, &attached))
        }
        Found::Dependency(run) => Facts::dependency(run).is_some_and(|facts| judge::tools::over(standing, action, &facts)),
    })
}
