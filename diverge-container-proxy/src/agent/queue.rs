//! What the queue holds, and what the server's channels say to it.

use diverge_provider_sdk::shared::containers::enqueue;
use diverge_provider_sdk::shared::error::Error;
use rmcp::model::ContentBlock;
use tokio::sync::oneshot;

/// What became of a message, as the server's enqueue channel is
/// answered.
pub enum Fate {
    /// A loop took it: the one in flight, or the one it started.
    Delivered,
    /// The server withdrew it before a loop took it.
    Dequeued,
    /// The agent's server refused it, or no loop could start on it:
    /// its own words.
    Error(Error),
}

impl From<Fate> for enqueue::response::Frame {
    fn from(fate: Fate) -> Self {
        match fate {
            Fate::Delivered => enqueue::response::Frame::Delivered,
            Fate::Dequeued => enqueue::response::Frame::Dequeued,
            Fate::Error(error) => enqueue::response::Frame::Error(error),
        }
    }
}

/// One message in the queue, and where its fate goes.
pub struct Queued {
    /// The caller's key, by which a dequeue withdraws it.
    pub key: String,
    /// The message's content, in order.
    pub content: Vec<ContentBlock>,
    /// The enqueue channel waiting for the fate. A receiver that is
    /// gone is a server that left, and a fate nobody hears.
    pub fate: oneshot::Sender<Fate>,
}

/// What a server's channel asks of the driver.
pub enum Cmd {
    /// A message for the agent.
    Enqueue(Queued),
    /// Withdraw every message still waiting under a key, and say how
    /// many were.
    Dequeue {
        /// The key, as the enqueues gave it.
        key: String,
        /// Where the count and the loop's state go.
        reply: oneshot::Sender<DequeueReply>,
    },
}

/// The driver's answer to a dequeue.
pub struct DequeueReply {
    /// How many messages the queue held under the key and gave back.
    pub drained: usize,
    /// Whether a loop is running, and so may hold messages of its
    /// own for the agent's server to withdraw.
    pub active: bool,
}
