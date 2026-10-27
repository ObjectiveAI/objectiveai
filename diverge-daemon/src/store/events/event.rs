//! One event, as the store holds it.

use diverge_sdk::daemon::endpoints::events::{self, Party};

/// One edge of the graph: the sender, the receiver, and how many times
/// something has passed between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// The daemon's number for the edge, given when it was added: a
    /// later edge has a higher id.
    pub id: u64,
    /// Who sent.
    pub sender: Party,
    /// Who received.
    pub receiver: Party,
    /// How many times it has passed: `1` when the edge was added, and
    /// one more at every occurrence since.
    pub count: u64,
}

impl Event {
    /// The event as a list reports it: the same, member for member.
    pub fn report(&self) -> events::Event {
        events::Event {
            id: self.id,
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            count: self.count,
        }
    }
}
