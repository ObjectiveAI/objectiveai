//! One link of a creator chain.

use serde::{Deserialize, Serialize};

use super::{Agent, Client, Tool};

/// Who made a thing, one link of the chain [`creator`](super)
/// describes. JSON-tagged by `type`, `client`, `agent` or `tool`,
/// beside the variant's own members.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Creator {
    /// The client itself, over an endpoint. The first link of every
    /// chain, and never any other. See [`Client`].
    Client(Client),
    /// An agent of the client's. See [`Agent`].
    Agent(Agent),
    /// A tool of the client's, one it made. See [`Tool`].
    Tool(Tool),
}
