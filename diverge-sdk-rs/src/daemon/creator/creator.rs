//! Who made a thing.

use serde::{Deserialize, Serialize};

use super::{Agent, Client, Tool};

/// Who made a thing, as [`creator`](super) describes: the client, or
/// an agent or a tool of the client's. JSON-tagged by `type`,
/// `client`, `agent` or `tool`, beside the variant's own members.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Creator {
    /// The client itself, over an endpoint. See [`Client`].
    Client(Client),
    /// An agent of the client's. See [`Agent`].
    Agent(Agent),
    /// A tool of the client's, one it made. See [`Tool`].
    Tool(Tool),
}
