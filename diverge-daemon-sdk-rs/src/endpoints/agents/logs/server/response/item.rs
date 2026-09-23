//! What one log entry holds.

use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use serde::{Deserialize, Serialize};

use super::Error;

/// What the log kept: one chunk of the agent's conversation, or one
/// error a run answered with.
///
/// Flattened into the [`ItemWrapper`](super::ItemWrapper) that holds
/// it, and every kind carries a `type` that names it — a chunk its
/// own, an error the string `error` — so the `type` alone says what
/// an item is, and the request's [`type`] picks by it. An error's
/// value lands whole under one member, `error`, because it is an
/// arbitrary JSON value and its members are nobody's promise; a
/// reader never looks inside it to tell an error from a chunk.
///
/// [`type`]: crate::endpoints::agents::logs::client::request::Frame::type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Item {
    /// One chunk, as the run streamed it — a user part, what the
    /// agent said, a tool call or its answer, usage, a notification.
    /// Its members are the item's.
    Chunk(AgenticLoopChunk),
    /// One error a run answered with: the run that would not start,
    /// or the message the agent refused, in the words the daemon
    /// received.
    Error(Error),
}
