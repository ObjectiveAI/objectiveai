//! The connected tool a request names: a record, and whether the
//! grants reach it.

use diverge_sdk::daemon::grant::connected_standalone_tools::Over;
use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use super::{active, agents_of, deployed, templates_of};
use crate::daemon::Daemon;
use crate::judge::filter::connected_standalone_tools::Facts;
use crate::judge::{self, Standing};
use crate::store::connected_standalone_tools::ConnectedTool;
use crate::store::{self, connected_standalone_tools};

/// The tool the reference names, if any: by name, or by daemon, tool
/// and index, a record, locked for the transaction when `lock`.
pub async fn resolve(conn: &mut PgConnection, reference: &reference::ConnectedStandaloneTool, lock: bool) -> Result<Option<ConnectedTool>, store::Error> {
    connected_standalone_tools::by_reference(conn, reference, lock).await
}

/// What a filter and a grant see of the record, loaded: whether it is
/// active now, whether its connect is held, and what it is attached
/// to — agents and agent templates.
pub struct Loaded {
    /// Whether it is active now.
    pub active: bool,
    /// Whether its connect is held.
    pub deployed: bool,
    /// The agents it is attached to directly.
    pub agents: Vec<diverge_sdk::daemon::key::Agent>,
    /// The agent templates it is attached to.
    pub templates: Vec<String>,
}

impl Loaded {
    /// The facts of `tool`, as loaded.
    pub fn facts<'a>(&'a self, tool: &'a ConnectedTool) -> Facts<'a> {
        Facts {
            tool,
            active: self.active,
            deployed: self.deployed,
            attached_agent_instances: &self.agents,
            attached_agent_templates: &self.templates,
        }
    }
}

/// The record's [`Loaded`], read now.
pub async fn load(conn: &mut PgConnection, daemon: &Daemon, tool: &ConnectedTool) -> Result<Loaded, store::Error> {
    Ok(Loaded {
        active: active(daemon, tool.id).await,
        deployed: deployed(daemon, tool.id).await,
        agents: agents_of(conn, tool.id).await?,
        templates: templates_of(conn, tool.id).await?,
    })
}

/// Whether the standing may do `action` to the record, its facts
/// loaded here.
pub async fn reaches(conn: &mut PgConnection, daemon: &Daemon, standing: &Standing, action: Over, tool: &ConnectedTool) -> Result<bool, store::Error> {
    let loaded = load(conn, daemon, tool).await?;
    Ok(judge::connected_standalone_tools::over(standing, action, &loaded.facts(tool)))
}
