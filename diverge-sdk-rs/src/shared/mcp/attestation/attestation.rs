//! The one object under the key: an agent, or a tool.

use serde::{Deserialize, Serialize};

use super::{Agent, Tool};
use crate::shared::containers::request::Image;

/// What is under [`KEY`](crate::shared::mcp::KEY): the [`Agent`] or
/// the [`Tool`] on the other side, each tagged by its own `kind`, so
/// the one object reads as whichever it is. What the daemon writes;
/// what a reader that may receive either holds — a tool container
/// reads the agent alone through [`request`](super::request), an
/// agent container the tool alone through
/// [`response`](super::response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
pub enum Attestation {
    /// An agent: `kind` is `agent`.
    Agent(Agent),
    /// A tool: `kind` is `created`, `connected` or `dependency`.
    Tool(Tool),
}

impl Attestation {
    /// The image of the container on the other side: an agent's
    /// always, a tool's but a connected one's.
    pub fn image(&self) -> Option<&Image> {
        match self {
            Attestation::Agent(agent) => Some(&agent.image),
            Attestation::Tool(tool) => tool.image(),
        }
    }
}
