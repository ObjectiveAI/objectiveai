//! The slot's fate, as the program states it.

use serde::{Deserialize, Serialize};

/// What became of the slot's message — the whole batch, every message
/// in it alike: the JSON body of a `2xx` from `POST /enqueue`, sent
/// when the fate is known and not before.
///
/// ```json
/// {"type": "delivered"} | {"type": "dequeued"} | {"type": "missed"}
/// ```
///
/// Only these three: an error is not a fate, and the program says it
/// as a non-`2xx` — a `4xx` for a batch it will not take, whose
/// content it cannot convert or which finds the slot occupied, which
/// the proxy answers on the wire as the error of every message in it
/// and never offers again; a `5xx` for a failure of its own, which
/// hands the batch back. `delivered` is every message's fate on the
/// provider's wire as well. `dequeued` and `missed` are not: the
/// batch is the proxy's again, and the proxy answers `dequeued` only
/// for the messages the provider withdrew, and offers the rest again
/// — to this loop after a `dequeued`, to the next after a `missed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Fate {
    /// The loop took the message into the conversation, as one user
    /// turn.
    Delivered,
    /// A `POST /dequeue` emptied the slot before the loop took it.
    Dequeued,
    /// The loop ended before the message could be taken, or none was
    /// running.
    Missed,
}
