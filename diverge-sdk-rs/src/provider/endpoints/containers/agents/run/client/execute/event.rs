//! One thing the agent's main stream says: a chunk, or the proxy's
//! word that a loop began or ended.

use crate::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;

/// What a reader of an agent's conversation gets, one at a time: the
/// three things the main stream carries after its opening, as
/// [`Frame`](crate::provider::endpoints::containers::agents::run::server::response::Frame)
/// carries them — a chunk the agent produced, or the proxy's own
/// [`Active`](Self::Active) before a loop's first chunk and
/// [`Inactive`](Self::Inactive) after its last.
///
/// One type for both layers: the provider reads it off the proxy's
/// begin stream, and the caller reads it off the run's, so a relay
/// between the two is a match with three arms and no translation.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// One chunk of the agent's conversation, within a loop.
    Chunk(AgenticLoopChunk),
    /// A loop began: the agent's server took the message and is
    /// working. Every chunk until the matching `Inactive` is this
    /// loop's.
    Active,
    /// A loop ended, however it ended. The agent is quiet until the
    /// next message.
    Inactive,
}
