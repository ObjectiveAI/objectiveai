//! What is done with the events.

use serde::{Deserialize, Serialize};

/// The actions over the events, each the endpoint of the same name
/// under [`events`](crate::daemon::endpoints::events).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// List the events, narrowed, and keep the list.
    List,
}
