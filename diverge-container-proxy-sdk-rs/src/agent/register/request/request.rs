//! The agent, as the request that made the container carried it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The agent, handed to the program once: the `agent` of the request
/// that made the container, typed to the same depth for the same
/// reason — a JSON value, because the image defines what an agent
/// is, and what the value may be is what `GET /schema` answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Request {
    /// The agent, as the image defines it.
    pub agent: Value,
}
