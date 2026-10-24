//! What passed between a sender and a receiver.

use serde::{Deserialize, Serialize};

/// The three things the daemon records passing from one party to
/// another, each at the seam it passes through. Snake case on the
/// wire: `"message"`, `"call"`, `"read"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// A message enqueued to an agent, whoever enqueued it: the client
    /// over [`agents::message`](crate::daemon::endpoints::agents::message),
    /// or an agent or a tool of its own. The receiver is the agent.
    Message,
    /// An MCP tool call into a tool. The receiver is the tool.
    Call,
    /// An MCP resource read into a tool. The receiver is the tool.
    Read,
}
