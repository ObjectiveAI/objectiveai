//! The enqueued message's fate, as the agent's server states it.

use serde::{Deserialize, Serialize};

use crate::shared::containers::enqueue::response;

/// What became of an enqueued message: the JSON body of a `2xx` from
/// `POST /enqueue`, sent when the fate is known and not before.
///
/// ```json
/// {"type": "delivered"} | {"type": "dequeued"} | {"type": "missed"}
/// ```
///
/// The three fates of the wire's
/// [`enqueue::response::Frame`](response::Frame), as JSON — and only
/// those three: an error is not a fate, and the agent's server says
/// it as a non-`2xx`, which the proxy forwards as the frame's
/// `Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Fate {
    /// The agent took the message into the conversation.
    Delivered,
    /// The caller withdrew the message before the agent took it.
    Dequeued,
    /// The run ended before the message could be taken, or none was
    /// running.
    Missed,
}

impl From<Fate> for response::Frame {
    fn from(fate: Fate) -> Self {
        match fate {
            Fate::Delivered => response::Frame::Delivered,
            Fate::Dequeued => response::Frame::Dequeued,
            Fate::Missed => response::Frame::Missed,
        }
    }
}
