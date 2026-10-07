//! Which container: an agent or a tool, by record.

use diverge_sdk::daemon::creator::{self, Creator};

use crate::store::agents::Agent;
use crate::store::tools::Tool;
use crate::store::{AgentId, ToolId};

/// One container of the daemon's, by the record it is made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// An agent.
    Agent(AgentId),
    /// A tool.
    Tool(ToolId),
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

/// A creator's snapshot of an agent, for the deployer wait.
pub fn agent_snapshot(agent: &Agent) -> creator::Agent {
    agent.snapshot()
}
