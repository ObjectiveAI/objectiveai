//! One link of a creator chain.

use serde::{Deserialize, Serialize};

use super::ToolOrigin;

/// Who made a thing, one link of the chain [`creator`](super)
/// describes. JSON-tagged by `type`: `client`, `agent` or `tool`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Creator {
    /// The client itself, over an endpoint. The first link of every
    /// chain, and never any other.
    Client {
        /// The client's identity, as the daemon holds the connection
        /// the create arrived on. The daemon's word for a caller,
        /// compared and not read.
        identity: String,
    },
    /// An agent of the client's.
    Agent {
        /// The template the agent was made from, by id.
        template: String,
        /// The agent's number among all agents of the client's ever
        /// made from that template: the
        /// [`count`](crate::daemon::endpoints::agents::list::server::response::Agent::count)
        /// its list item carries. The template and the count name the
        /// agent once and for all.
        count: u64,
        /// The agent's name, as its create gave it.
        name: String,
    },
    /// A tool of the client's.
    Tool {
        /// What the tool was made with: see [`ToolOrigin`].
        origin: ToolOrigin,
        /// The tool's number among all tools of the client's ever
        /// made with that origin: the
        /// [`count`](crate::daemon::endpoints::tools::list::server::response::Tool::count)
        /// its list item carries. The origin and the count name the
        /// tool once and for all.
        count: u64,
        /// The tool's name, as its create or its connect gave it.
        name: String,
    },
}
