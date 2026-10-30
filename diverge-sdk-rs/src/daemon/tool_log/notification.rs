//! A notification a tool sent, as the tool's log keeps it.

use serde::{Deserialize, Serialize};

/// One notification the tool sent, whole, as the tool sent it: MCP's
/// own JSON — `method` and `params` — kept as it came, so a
/// notification this crate does not know is kept as well as one it
/// does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Notification {
    /// The notification.
    pub notification: serde_json::Value,
}
