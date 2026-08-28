//! What became of the enqueued message.

use serde::{Deserialize, Serialize};

/// The message's fate — the whole of the response, arriving only
/// when the fate is known.
///
/// **Untagged, discriminated by payload**, the way this crate's JSON
/// unions are: every variant carries a `type` field whose value no
/// other variant can produce. The fates carry nothing else — the
/// fate IS the answer, and the message's content is the caller's
/// own to remember. There is no error among them: a container that
/// cannot answer says so as HTTP does, with a non-2xx status, the
/// same way the run request already fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// The agent took the message into the conversation.
    ///
    /// Its mark in the response stream is the
    /// [`user`](crate::endpoints::agentic_loop::run::server::response::UserChunk)
    /// chunk carrying this prompt at the position it landed.
    Delivered {
        /// Always `delivered`.
        r#type: DeliveredType,
    },
    /// The caller withdrew the message before the agent took it.
    Dequeued {
        /// Always `dequeued`.
        r#type: DequeuedType,
    },
    /// The run ended before the message could be taken.
    ///
    /// Nothing malfunctioned and nobody withdrew it — the
    /// conversation finished first, and the message will never enter
    /// it. A caller that still wants it heard sends it as the next
    /// run's prompt.
    Missed {
        /// Always `missed`.
        r#type: MissedType,
    },
}

/// The `delivered` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DeliveredType {
    /// The only value.
    #[default]
    Delivered,
}

/// The `dequeued` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DequeuedType {
    /// The only value.
    #[default]
    Dequeued,
}

/// The `missed` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MissedType {
    /// The only value.
    #[default]
    Missed,
}
