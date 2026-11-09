//! Which container: an agent or a tool, by record or by deployment.

use diverge_sdk::daemon::creator::Creator;

use crate::store::agents::Agent;
use crate::store::connected_standalone_tools::ConnectedTool;
use crate::store::tools::Tool;
use crate::store::{AgentId, ConnectedToolId, DependencyToolId, ToolId};

/// One tool run of the daemon's: a standalone tool's, a connected
/// tool's — the connect this daemon holds on another daemon's tool —
/// or a dependency tool's, by its row — a transient tool's for its
/// agent's run, a service's for as long as the daemon keeps it up; at
/// most one run per row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKey {
    /// A standalone tool on record, by its row.
    Record(ToolId),
    /// A connected standalone tool, by its row.
    Connected(ConnectedToolId),
    /// A dependency tool, by its row.
    Dependency(DependencyToolId),
}

/// A connect scope's number among those another daemon has opened
/// since the daemon started: minted by [`Live`](crate::daemon::Live),
/// never given twice. What holds a tool's run up for the scope's
/// life — a real connection, beside the real use that is an exchange
/// in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectionId(pub u64);

/// One container of the daemon's: an agent, by the record it is made
/// from; or a tool, by its record or by its dependency tool's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// An agent.
    Agent(AgentId),
    /// A tool.
    Tool(ToolKey),
}

/// What a container is as a maker and as a sender: the agent's or the
/// tool's template, index and name, which is what its log items and
/// its messages carry.
pub fn sender_of_agent(agent: &Agent) -> Creator {
    Creator::Agent(agent.snapshot())
}

/// The tool as a sender: its template, index and name.
pub fn sender_of_tool(tool: &Tool) -> Creator {
    Creator::Tool(tool.snapshot())
}

/// A connected tool as a sender: never a party of its own — another
/// daemon's tool, it has no row in the graph — so it stands as its
/// creator, who made it here.
pub fn sender_of_connected(tool: &ConnectedTool) -> Creator {
    tool.creator.clone()
}

/// The serve-name of an attached tool in an agent's tool listing: its
/// name as it is called, else the first eight hex digits of its
/// template and its index.
pub fn serve_name(tool: &Tool) -> String {
    if let Some(name) = &tool.name {
        return name.clone();
    }
    format!("{}-{}", tool.template.chars().take(8).collect::<String>(), tool.index)
}

/// The serve-name of an attached connected tool: its name as it is
/// called, else `connected-` and its daemon record's ID and its index,
/// which no standalone tool's serve-name can be.
pub fn serve_name_connected(tool: &ConnectedTool) -> String {
    if let Some(name) = &tool.name {
        return name.clone();
    }
    format!("connected-{}-{}", tool.daemon, tool.index)
}
