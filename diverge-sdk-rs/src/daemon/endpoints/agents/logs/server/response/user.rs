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
/// # The sender
///
/// `sender` is one [`Creator`](crate::daemon::creator::Creator): the
/// client, for a message it sent with
/// [`message`](crate::daemon::endpoints::agents::message); the agent
/// or the tool, for one sent through the daemon's own `agents_message`
/// tool. The message request names no sender: the daemon knows who
/// sent from the scope the request arrived on, and keeps it here.
/// Nothing but a user part has a sender.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Who sent the message this part is of: one
    /// [`Creator`](crate::daemon::creator::Creator).
    pub sender: Creator,
    /// The part, as the run streamed it: one of the five `user_*`
    /// chunks, its members beside `sender`.
    #[serde(flatten)]
    pub chunk: AgenticLoopChunk,
}
