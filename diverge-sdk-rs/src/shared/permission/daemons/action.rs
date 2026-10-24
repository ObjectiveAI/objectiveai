//! What may be done through remote daemons.

use serde::{Deserialize, Serialize};

/// An act over the daemons this daemon reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Read one, by name.
    Get,
    /// List them.
    List,
    /// Connect a tool of one's to this daemon.
    Connect,
    /// Relay a request to one, served there.
    Relay,
}
