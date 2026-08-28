//! What a dequeue asks.

use serde::{Deserialize, Serialize};

/// Withdraw every message still waiting in the queue.
///
/// The same statement the channel-level
/// [`Dequeue`](crate::endpoints::agentic_loop::run::client::channel_request::Dequeue)
/// makes, at the container's door. It carries nothing — the whole
/// queue is the only thing withdrawable, because enqueued messages
/// have no identifiers to withdraw one by — and it exists as a type
/// anyway so the route has a named body: an empty object, `{}`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize,
)]
pub struct Request {}
