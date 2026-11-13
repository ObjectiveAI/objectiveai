//! What one log entry holds.

use crate::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use serde::{Deserialize, Serialize};

use super::{Active, Deployed, Error, Inactive, StructuredDecisionsResponse, Undeployed, User, UserStructuredDecisions};

/// What the log kept: one user part of a message with who sent it,
/// one other chunk of the agent's conversation, one error a run
/// answered with, or one step of the run's life — see
/// [`lifecycle`](crate::daemon::lifecycle).
///
/// Flattened into the [`ItemWrapper`](super::ItemWrapper) that holds
/// it, and every kind carries a `type` that names it — a chunk its
/// own, the others the strings `error`, `deployed`, `active`,
/// `inactive` and `undeployed` — so
/// the `type` alone says what an item is, and the request's
/// [`type`] picks by it. An error's value lands whole under one
/// member, `error`, because it is an arbitrary JSON value and its
/// members are nobody's promise; a reader never looks inside it to
/// tell an error from a chunk. The provider's members lie beside a
/// `deployed` or `undeployed` item's own. A user part — a chunk whose
/// `type` is one of the five `user_*` — is a [`User`](Self::User)
/// and carries `sender` beside its members; no other item does, and
/// `sender` is what tells a user part from any other chunk.
///
/// [`type`]: crate::daemon::endpoints::logs::client::request::Frame::type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
pub enum Item {
    /// A structured decision a caller sent, numbered. Before
    /// [`User`](Self::User) and [`Chunk`](Self::Chunk), which would
    /// each read one too and lose its id: the enum is untagged and
    /// takes the first that fits.
    UserStructuredDecisions(UserStructuredDecisions),
    /// An agent's answers to a structured decision, numbered. Before
    /// [`Chunk`](Self::Chunk), for the same reason.
    StructuredDecisionsResponse(StructuredDecisionsResponse),
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
    /// The agent's container came up on a provider, and which.
    Deployed(Deployed),
    /// The agent began working.
    Active(Active),
    /// The agent ceased working.
    Inactive(Inactive),
    /// The agent's container went down, and why when unexpectedly.
    Undeployed(Undeployed),
}
