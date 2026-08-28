//! What the clearing found.

use serde::{Deserialize, Serialize};

use crate::agentic_loop_container::enqueue;

/// Whether the queue held anything — the clearing's summary, not a
/// restatement of the messages: each withdrawn message's own enqueue
/// answers [`dequeued`](super::super::enqueue::Response::Dequeued).
///
/// **Untagged, discriminated by payload**, like every JSON union
/// here: each variant carries a `type` no other variant can match.
/// No error among the variants: a container that cannot answer says
/// so as HTTP does, with a non-2xx status.
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
