//! What a dependency tool is attached to, whose it is and whether it
//! runs, loaded.

use diverge_sdk::daemon::agent_log::Provider;
use diverge_sdk::daemon::endpoints::dependency_tools::list::server::response::{Attached, Listed, Run};
use diverge_sdk::daemon::grant::dependency_tools::Over;
use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use crate::containers::{ToolKey, ToolRun};
use crate::daemon::Daemon;
use crate::judge::filter::dependency_tools::Facts;
use crate::judge::{self, Standing};
use crate::store::dependency_tools::DependencyTool;
use crate::store::{self, agents, dependency_tools};

/// What a row does not carry: whose the tool is, as the wire names
/// it, the agents attached to it, and its run.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// Whose it is.
    pub owner: key::Owner,
    /// The agents attached, in order, each with when it last declared
    /// the template.
    pub attached: Vec<Attached>,
    /// Its run, while its container is up.
    pub run: Option<Run>,
    /// Whether it is working now: a call or a read in flight.
    pub active: bool,
}

impl Loaded {
    /// The facts a grant and a filter judge the tool by.
    pub fn facts<'a>(&'a self, tool: &'a DependencyTool) -> Facts<'a> {
        Facts {
            tool,
            owner: &self.owner,
            attached: &self.attached,
            run: self.run.as_ref(),
        }
    }

    /// The tool as a list reports it.
    pub fn report(self, tool: &DependencyTool) -> Listed {
        tool.report(self.owner, self.attached, self.active, self.run)
    }
}

/// The run as the list reports it: where it runs, and under what id.
pub fn run_of(run: &ToolRun) -> Run {
    Run {
        provider: Provider {
            identity: run.provider.clone(),
        },
        id: run.container.clone().unwrap_or_default(),
    }
}

/// The tool's owner, attachments and run, loaded now. `None` for a
/// per-agent tool whose agent is gone, which is a row on its way out.
pub async fn load(conn: &mut PgConnection, daemon: &Daemon, tool: &DependencyTool) -> Result<Option<Loaded>, store::Error> {
    let owner_agent = match tool.owner_agent {
        Some(id) => agents::by_id(conn, id, false).await?,
        None => None,
    };
    let Some(owner) = tool.owner(owner_agent.as_ref()) else {
        return Ok(None);
    };
    let mut attached = Vec::new();
    for attachment in dependency_tools::attachments::attached(conn, tool.id).await? {
        if let Some(agent) = agents::by_id(conn, attachment.agent, false).await? {
            attached.push(Attached {
                agent: agent.key(),
                declared: attachment.declared,
            });
        }
    }
    let run = daemon.live.tool_run(ToolKey::Dependency(tool.id)).await.map(|run| run_of(&run));
    let active = daemon.live.is_busy(ToolKey::Dependency(tool.id)).await;
    Ok(Some(Loaded { owner, attached, run, active }))
}

/// Whether the standing may do `action` to the tool: its owner, its
/// attachments and its run loaded here. A tool whose owner agent is
/// gone is reached by nothing.
pub async fn reaches(conn: &mut PgConnection, daemon: &Daemon, standing: &Standing, action: Over, tool: &DependencyTool) -> Result<bool, store::Error> {
    Ok(match load(conn, daemon, tool).await? {
        Some(loaded) => judge::dependency_tools::over(standing, action, &loaded.facts(tool)),
        None => false,
    })
}
