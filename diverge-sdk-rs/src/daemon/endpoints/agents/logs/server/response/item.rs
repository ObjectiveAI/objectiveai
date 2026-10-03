//! What one log entry holds.

use crate::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use serde::{Deserialize, Serialize};

use super::{Active, Error, Inactive, User};

/// What the log kept: one user part of a message with who sent it,
/// one other chunk of the agent's conversation, one error a run
/// answered with, or the agent starting or ceasing to run on a
/// provider.
///
/// Flattened into the [`ItemWrapper`](super::ItemWrapper) that holds
/// it, and every kind carries a `type` that names it — a chunk its
/// own, the others the strings `error`, `active` and `inactive` — so
/// the `type` alone says what an item is, and the request's
/// [`type`] picks by it. An error's value lands whole under one
/// member, `error`, because it is an arbitrary JSON value and its
/// members are nobody's promise; a reader never looks inside it to
/// tell an error from a chunk. The provider's members lie beside an
/// `active` or `inactive` item's own. A user part — a chunk whose
/// `type` is one of the five `user_*` — is a [`User`](Self::User)
/// and carries `sender` beside its members; no other item does, and
/// `sender` is what tells a user part from any other chunk.
///
/// [`type`]: crate::daemon::endpoints::agents::logs::client::request::Frame::type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Item {
    /// One user part of a message that landed, with who sent the
    /// message. Its members, and `sender`, are the item's.
    User(User),
    /// One other chunk, as the run streamed it — what the agent said,
    /// a tool call or its answer, usage, a notification. Its members
    /// are the item's.
    Chunk(AgenticLoopChunk),
    /// One error a run answered with: the run that would not start,
    /// or the message the agent refused, in the words the daemon
    /// received.
    Error(Error),
    /// The agent began running on a provider, and which.
    Active(Active),
    /// The agent ceased running on a provider, and which.
    Inactive(Inactive),
}
