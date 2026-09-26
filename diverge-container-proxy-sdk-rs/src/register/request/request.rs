//! The arguments, as the request that made the container carried
//! them.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The arguments, handed to the program once: the `arguments` of the
/// request that made the container, typed to the same depth for the
/// same reason — a JSON value, because the image defines what it
/// takes, and what the value may be is what `GET /schema` answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Request {
    /// The arguments, as the image defines them.
    pub arguments: Value,
}
