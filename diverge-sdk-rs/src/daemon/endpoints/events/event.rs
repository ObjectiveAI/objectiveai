//! One edge of the graph: who sent, who received, what, how often.

use serde::{Deserialize, Serialize};

use super::Action;
use crate::daemon::creator::Creator;

/// One event, as a list reports it: the sender, the receiver, the
/// [`Action`] between them, and how many times it has passed. One
/// per sender, receiver and action, as [`events`](super) states; the
/// id is the daemon's, given once when the edge is added and never
/// again, so the latest edges are the highest, and a list's window
/// is cut by it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Event {
    /// The daemon's number for this edge, given when it was added:
    /// a later edge has a higher id, and nothing else is read into
    /// it.
    pub id: u64,
    /// Who sent: the client over an endpoint, or the agent or the
    /// tool that acted. One
    /// [`Creator`](crate::daemon::creator::Creator).
    pub sender: Creator,
    /// Who received: the agent a message was enqueued to, or the
    /// tool an MCP call or read went into. One
    /// [`Creator`](crate::daemon::creator::Creator).
    pub receiver: Creator,
    /// What passed: see [`Action`].
    pub action: Action,
    /// How many times it has passed: `1` when the edge is added, and
    /// one more at every occurrence since. Never `0`, and never less
    /// than it was.
    pub count: u64,
}
