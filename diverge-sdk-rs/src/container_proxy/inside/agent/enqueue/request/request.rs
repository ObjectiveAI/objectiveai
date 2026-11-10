//! What the program's slot is asked to hold.

use serde::{Deserialize, Serialize};

use crate::container_proxy::inside::agent::run::request::Message;

/// Hold these messages, as ONE: the program's one slot filled.
///
/// The messages are the proxy's whole queue as it offers it — one or
/// more, in the order they were enqueued, each under the key its
/// enqueue carried. The program holds them as one message and takes
/// them as one user turn: the content of every message, in order,
/// and the user parts of each under its own key, so every sender is
/// still credited in the log. A program that cannot take a block
/// answers a `4xx` — the batch refused whole, in its own words — and
/// the proxy answers every message in it with them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// What the slot holds, in enqueue order.
    pub messages: Vec<Message>,
}
