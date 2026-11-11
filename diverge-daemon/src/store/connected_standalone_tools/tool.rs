//! One connected tool, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::connected_standalone_tools::list::server::response::ConnectedStandaloneTool;
use diverge_sdk::daemon::endpoints::daemons::DaemonId;
use diverge_sdk::daemon::key;
use diverge_sdk::daemon::reference;

use crate::store::ConnectedToolId;

/// A connected tool row: another daemon's standalone tool, held here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedTool {
    /// The row.
    pub id: ConnectedToolId,
    /// The remote daemon the tool is on, by the ID of this daemon's
    /// record of it — the first of the route's.
    pub daemon: DaemonId,
    /// The records the connect is relayed through beyond `daemon`, each
    /// by its ID on the daemon before it; empty for a tool on `daemon`
    /// itself.
    pub route: Vec<DaemonId>,
    /// The tool, as that daemon names it.
    pub tool: reference::Tool,
    /// Its index among the connected tools ever held from that
    /// daemon's tool.
    pub index: u64,
    /// The name, if the tool has one.
    pub name: Option<String>,
    /// When its connect was last taken or let go.
    pub last_deployed: Option<DateTime<Utc>>,
    /// When its calls in flight last began or ceased.
    pub last_active: Option<DateTime<Utc>>,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When it was made.
    pub created: DateTime<Utc>,
    /// Who made it.
    pub creator: Creator,
}

impl ConnectedTool {
    /// The tool as a key: the daemon, the tool there, its index, and its
    /// name as it is called now.
    pub fn key(&self) -> key::Tool {
        key::Tool::ConnectedStandalone {
            daemon: self.daemon,
            tool: self.tool.clone(),
            index: self.index,
            name: self.name.clone(),
        }
    }

    /// The record as a list reports it, given whether it is active now
    /// (a call in flight), whether its connect is held, and the agents
    /// and the agent templates it is attached to.
    pub fn report(&self, active: bool, deployed: bool, agents: Vec<key::Agent>, templates: Vec<String>) -> ConnectedStandaloneTool {
        ConnectedStandaloneTool {
            name: self.name.clone(),
            daemon: self.daemon,
            route: self.route.clone(),
            tool: self.tool.clone(),
            index: self.index,
            creator: self.creator.clone(),
            created: self.created,
            active,
            last_active: self.last_active,
            deployed,
            last_deployed: self.last_deployed,
            attached_agent_instances: agents,
            attached_agent_templates: templates,
            tags: self.tags.clone(),
        }
    }
}
