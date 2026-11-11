//! One edge of the graph: who sent, who received, how often.

use serde::{Deserialize, Serialize};

use crate::shared::listing::Keyed;
use super::Party;

/// One event, as a list reports it: the sender, the receiver, and how
/// many times something has passed between them. One per sender and
/// receiver, as [`events`](super) states; the id is the daemon's,
/// given once when the edge is added and never again, so the latest
/// edges are the highest, and a list's window is cut by it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Event {
    /// The daemon's number for this edge, given when it was added:
    /// a later edge has a higher id, and nothing else is read into
    /// it.
    pub id: u64,
    /// Who sent: the account over an endpoint, or the agent or the
    /// tool that acted. One [`Party`].
    pub sender: Party,
    /// Who received: the agent a message was enqueued to, or the
    /// tool an MCP call or read went into. One [`Party`].
    pub receiver: Party,
    /// How many times it has passed: `1` when the edge is added, and
    /// one more at every occurrence since. Never `0`, and never less
    /// than it was.
    pub count: u64,
}

/// Keyed by the id: a `Changed` event is the whole record with its
/// count raised, and a `Removed` one the record as it was last sent;
/// each lands on the one with its id.
impl Keyed for Event {
    type Key = u64;

    fn key(&self) -> u64 {
        self.id
    }
}
