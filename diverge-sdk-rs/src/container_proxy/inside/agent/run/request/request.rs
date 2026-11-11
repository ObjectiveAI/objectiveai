//! What one loop is asked.

use serde::{Deserialize, Serialize};

use super::Message;

/// Run one loop on these messages.
///
/// The arguments are not here: they were registered once, and they
/// never change. The messages are each loop's, because a container
/// runs loops one after another — each resuming the conversation the
/// last one left — and every one is asked something: one or more
/// messages, in the order their batches carried them, each under the
/// key its enqueue carried. The program joins them into ONE user turn — the
/// content of every message, in order — and yields each message's
/// user parts, under its own key, in order, as the stream's first
/// chunks, before anything else it says.
/// A program that cannot take a block answers a `4xx` — every
/// message refused, in its own words — and the proxy answers every
/// message the loop was to take with them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// What the loop is asked, in order: every batch the proxy holds,
    /// in the order the batches reached it, each batch's messages in
    /// the batch's own order.
    pub messages: Vec<Message>,
}
