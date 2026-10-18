//! One event, as the store holds it.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::events::{self, Action};

/// One edge of the graph: the sender, the receiver, the action between
/// them, and how many times it has passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// The daemon's number for the edge, given when it was added: a
    /// later edge has a higher id.
    pub id: u64,
    /// Who sent.
    pub sender: Creator,
    /// Who received.
    pub receiver: Creator,
    /// What passed between them.
    pub action: Action,
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
            action: self.action,
            count: self.count,
        }
    }
}

/// The action's word, as the `action` column holds it: the snake case
/// of the wire, and what the column's constraint admits.
pub fn action_name(action: Action) -> &'static str {
    match action {
        Action::Message => "message",
        Action::Call => "call",
        Action::Read => "read",
    }
}

/// The action a column's word names, or `None` for a word that is
/// none of the three.
pub fn action_of(name: &str) -> Option<Action> {
    match name {
        "message" => Some(Action::Message),
        "call" => Some(Action::Call),
        "read" => Some(Action::Read),
        _ => None,
    }
}
