//! What one log item holds.

use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use diverge_provider_sdk::shared::error::Error;
use serde::{Deserialize, Serialize};

/// What the log kept: one chunk of the agent's conversation, or one
/// error a run answered with.
///
/// Untagged, and unambiguous without a tag: a chunk serializes as a
/// JSON object with a `type` member no error carries, since an error
/// is the protocol's one error shape, `{"kind": …, …}`. A chunk is
/// tried first, and a value that is neither fails to read rather
/// than arriving as data nobody checks. Flattened into the
/// [`Item`](super::Item) that holds it, so the item IS the chunk or
/// the error, with the log's two members beside.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Kept {
    /// One chunk, as the run streamed it — a user part, what the
    /// agent said, a tool call or its answer, usage, a notification.
    Chunk(AgenticLoopChunk),
    /// One error a run answered with: the run that would not start,
    /// or the message the agent refused, in the words the daemon
    /// received.
    Error(Error),
}
