//! What the clearing found, as the program states it.

use serde::{Deserialize, Serialize};

/// Whether the program's slot held anything: the JSON body of a `2xx`
/// from `POST /dequeue`.
///
/// ```json
/// {"type": "dequeued"} | {"type": "empty"}
/// ```
///
/// Only these two: a failure is a non-`2xx`. The proxy answers the
/// provider from the held `/enqueue`'s fate, which this only hastens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Outcome {
    /// The slot held a message, and it is withdrawn.
    Dequeued,
    /// The slot held nothing: empty, or already taken.
    Empty,
}
