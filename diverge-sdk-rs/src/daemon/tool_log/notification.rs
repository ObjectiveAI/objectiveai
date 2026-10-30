//! A notification a tool sent, as the tool's log keeps it.

use serde::{Deserialize, Serialize};

use crate::shared::mcp::attestation;

/// One notification the tool sent, whole, as the tool sent it: MCP's
/// own JSON — `method` and `params` — kept as it came, so a
/// notification this crate does not know is kept as well as one it
/// does; and whom it reached — the caller whose MCP session the tool
/// sent it on (see [`session`](crate::shared::mcp::session)).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Notification {
    /// The caller whose session it was sent on.
    pub to: attestation::Caller,
    /// The notification.
    pub notification: serde_json::Value,
}
