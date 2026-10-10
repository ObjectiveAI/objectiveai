//! Which container: an agent or a tool, by record or by deployment.

use diverge_sdk::daemon::creator::Creator;

use crate::store::agents::Agent;
use crate::store::tools::Tool;
use crate::store::{AgentId, ToolId};

/// A dependency tool's number among the dependencies deployed since
/// the daemon started: minted by [`Live`](crate::daemon::Live), never
/// given twice, nobody's record. What a dependency run is keyed by
/// while it lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DependencyId(pub u64);

/// One tool run of the daemon's: a record's, or a dependency's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKey {
    /// A tool on record, by its row.
    Record(ToolId),
    /// A dependency tool, by the number it was deployed under.
    Dependency(DependencyId),
}

/// An exposure's number among those opened since the daemon started:
/// minted by [`Live`](crate::daemon::Live), never given twice. What
/// holds a tool's run up for an expose scope's life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExposureId(pub u64);

/// One user of a tool's run: a container of the daemon's that calls
/// it or works on its files, or an expose scope that holds it for
/// another daemon to join. The run is stopped when no user remains
/// and no connector is attached from outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum User {
    /// A container: an agent whose loop runs or whose call is being
    /// served, or a container a file operation is on.
    Container(Key),
    /// An expose scope, for as long as it is open.
    Exposure(ExposureId),
}

/// One container of the daemon's: an agent, by the record it is made
/// from; or a tool, by its record or by its deployment.
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
