//! What the clearing found.

use serde::{Deserialize, Serialize};

use crate::queue::enqueue;
use crate::shared;

/// Whether the queue held anything — the clearing's summary, not a
/// restatement of the messages: each withdrawn message's own enqueue
/// answers [`dequeued`](super::super::enqueue::Response::Dequeued).
///
/// **Untagged, discriminated by payload**, like every JSON union
/// here: each variant carries a `type` no other variant can match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// The queue held messages, and they are withdrawn.
    Dequeued {
        /// Always `dequeued`.
        r#type: enqueue::DequeuedType,
    },
    /// The queue held nothing.
    ///
    /// Not a failure: everything previously enqueued had already
    /// been taken, withdrawn, or missed, and there was nothing left
    /// for the clearing to do.
    Empty {
        /// Always `empty`.
        r#type: EmptyType,
    },
    /// The queue's state could not be determined — the container's
    /// own failure, in the protocol's one error shape.
    Error {
        /// Always `error`.
        r#type: enqueue::ErrorType,
        /// What the container had to say.
        error: shared::error::Error,
    },
}

/// The `empty` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum EmptyType {
    /// The only value.
    #[default]
    Empty,
}
