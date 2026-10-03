//! One user part of a message, and who sent the message.

use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;

/// One user part the run streamed — a text, an image, an audio, a
/// resource or a resource link of a message that landed — with who
/// sent the message beside it. The part's own members are flattened
/// here, so the item reads as the chunk it is with `sender` added;
/// which message the part belongs to, its own `key` says.
///
/// # The sender is a chain
///
/// `sender` is the chain [`creator`](crate::daemon::creator)
/// describes, read the same way: the client first, each next
/// reached through the one before, the last the one that sent. A
/// message the client sent with
/// [`message`](crate::daemon::endpoints::agents::message) has a
/// chain of one. One an agent or a tool sent, through the daemon's
/// own `agents_message` tool, has that agent or tool last, and its
/// own makers before it. The message request names no sender: the
/// daemon knows who sent from the scope the request arrived on, and
/// keeps it here. Nothing but a user part has a sender.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Who sent the message this part is of, and through whom. Never
    /// empty.
    pub sender: Vec<Creator>,
    /// The part, as the run streamed it: one of the five `user_*`
    /// chunks, its members beside `sender`.
    #[serde(flatten)]
    pub chunk: AgenticLoopChunk,
}
