//! What the clearing found, as the agent's server states it.

use serde::{Deserialize, Serialize};

use crate::shared::containers::dequeue::response;

/// Whether the queue held anything: the JSON body of a `2xx` from
/// `POST /dequeue`.
///
/// ```json
/// {"type": "dequeued"} | {"type": "empty"}
/// ```
///
/// The two outcomes of the wire's
/// [`dequeue::response::Frame`](response::Frame), as JSON — and only
/// those two: a failure is a non-`2xx`, which the proxy forwards as
/// the frame's `Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Outcome {
    /// The queue held messages, and they are withdrawn.
    Dequeued,
    /// The queue held nothing.
    Empty,
}

impl From<Outcome> for response::Frame {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Dequeued => response::Frame::Dequeued,
            Outcome::Empty => response::Frame::Empty,
        }
    }
}
