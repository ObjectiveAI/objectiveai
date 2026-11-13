//! What one enqueue hands the agent: chat messages, or one decision.

use serde::{Deserialize, Serialize};

use super::Message;
use crate::shared::containers::structured_decisions::Decision;

/// What an enqueue hands the agent, and what a run inside the
/// container is started with: one or more chat messages, or exactly
/// one structured decision. Internally tagged by `type`, snake case:
/// `{"type":"chat","messages":[…]}`,
/// `{"type":"structured_decisions","key":…,"content":[…],"questions":[…]}`.
///
/// The two are handed over differently, and the container proxy is
/// what tells them apart: see [`Request`](super::Request).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    /// Chat: a batch of messages, in order, each under its key.
    Chat {
        /// The batch's messages, in order.
        messages: Vec<Message>,
    },
    /// One structured decision: see [`Decision`].
    StructuredDecisions(Decision),
}
