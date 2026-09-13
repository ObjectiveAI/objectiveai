//! What the clearing found, as the program states it.

use serde::{Deserialize, Serialize};

/// Whether the program's queue held anything: the JSON body of a
/// `2xx` from `POST /dequeue`.
///
/// ```json
/// {"type": "dequeued"} | {"type": "empty"}
/// ```
///
/// Only these two: a failure is a non-`2xx`. The proxy's answer to
/// the provider folds this in with what the proxy withdrew itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Outcome {
    /// The queue held messages, and they are withdrawn.
    Dequeued,
    /// The queue held nothing.
    Empty,
}
