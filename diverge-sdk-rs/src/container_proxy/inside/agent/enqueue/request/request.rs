//! What the program's queue is asked to take.

use serde::{Deserialize, Serialize};

use crate::container_proxy::inside::agent::run::request::Message;

/// Take these messages onto the queue.
///
/// The messages are the proxy's whole queue as it sends it — one or
/// more, in the order they were enqueued, each under the key its
/// enqueue carried. The program takes them, with anything else
/// waiting, as one user turn at its next seam: the content of every
/// message, in order, and the user parts of each under its own key,
/// so every sender is still credited in the log. A program that
/// cannot take a block answers a `4xx` before accepting any — the
/// batch refused whole, in its own words — and the proxy answers
/// every message in it with them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// What the queue takes, in index order: the daemon's arrival
    /// order, which every enqueue on the provider's wire carries.
    pub messages: Vec<Message>,
}
