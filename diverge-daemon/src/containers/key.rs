//! Which container: an agent or a tool, by record or by deployment.

use diverge_sdk::daemon::creator::Creator;

use crate::store::agents::Agent;
use crate::store::tools::Tool;
use crate::store::{AgentId, DependencyToolId, ToolId};

/// One tool run of the daemon's: a record's, or a dependency tool's,
/// by its row — a transient tool's for its agent's run, a service's
/// for as long as the daemon keeps it up; at most one run per row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKey {
    /// A tool on record, by its row.
    Record(ToolId),
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

/// The tool as a sender; a connected tool, made from no template,
/// names its runner's container instead through its creator.
pub fn sender_of_tool(tool: &Tool) -> Creator {
    match tool.snapshot() {
        Some(snapshot) => Creator::Tool(snapshot),
        None => tool.creator.clone(),
    }
}

/// The serve-name of an attached tool in an agent's tool listing: its
/// name as it is called, else the first eight hex digits of its
/// template and its index, or its connected id and index.
pub fn serve_name(tool: &Tool) -> String {
    if let Some(name) = &tool.name {
        return name.clone();
    }
    match tool.template() {
        Some(template) => format!("{}-{}", template.chars().take(8).collect::<String>(), tool.index),
        None => format!("connected-{}", tool.index),
    }
}
