//! What one log item holds.

use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use diverge_provider_sdk::shared::error::Error;
use serde::{Deserialize, Serialize};

/// What the log kept: one chunk of the agent's conversation, or one
/// error a run answered with.
///
/// Flattened into the [`Item`](super::Item) that holds it, and shaped
/// so that flattening is unambiguous: a chunk's own members land on
/// the item, its `type` among them; an error lands under one member,
/// `error`, whole, because an error is an arbitrary JSON value and
/// its members are nobody's promise. So an item with a `type` is a
/// chunk and an item with an `error` is an error, and a reader never
/// looks inside the error's value to tell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Kept {
    /// One chunk, as the run streamed it — a user part, what the
    /// agent said, a tool call or its answer, usage, a notification.
    /// Its members are the item's.
    Chunk(AgenticLoopChunk),
    /// One error a run answered with: the run that would not start,
    /// or the message the agent refused, in the words the daemon
    /// received.
    Error {
        /// The error, whole, under its own member.
        error: Error,
    },
}
