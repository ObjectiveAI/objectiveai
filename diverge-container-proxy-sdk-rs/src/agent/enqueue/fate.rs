//! The enqueued message's fate, as the program states it.

use serde::{Deserialize, Serialize};

/// What became of an enqueued message: the JSON body of a `2xx` from
/// `POST /enqueue`, sent when the fate is known and not before.
///
/// ```json
/// {"type": "delivered"} | {"type": "dequeued"} | {"type": "missed"}
/// ```
///
/// Only these three: an error is not a fate, and the program says it
/// as a non-`2xx`. The first two are the message's fate on the
/// provider's wire as well. The third is not: a message the loop
/// missed is the proxy's again, and starts the next loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Fate {
    /// The loop took the message into the conversation.
    Delivered,
    /// A `POST /dequeue` withdrew the message before the loop took
    /// it.
    Dequeued,
    /// The loop ended before the message could be taken, or none was
    /// running.
    Missed,
}
