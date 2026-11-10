//! What the queue holds, and what the server's channels say to it.

use diverge_sdk::shared::containers::{dequeue, enqueue};
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
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
    /// Withdraw every message still waiting under a key, and say
    /// whether any was, once that is exactly known.
    Dequeue {
        /// The key, as the enqueues gave it.
        key: String,
        /// Where the answer goes: at once when the offered batch does
        /// not hold the key, and once its outcome is known when it does.
        reply: oneshot::Sender<DequeueReply>,
    },
    /// The secrets, fed: the server's values for the program, which
    /// open the gate once the program has applied them.
    Feed {
        /// The values, by name.
        values: Values,
        /// Where the word that they were applied — or the program's
        /// refusal, in its own words — goes.
        reply: oneshot::Sender<Result<(), Error>>,
    },
}

/// The driver's answer to a dequeue, as the server's dequeue channel
/// is answered.
pub enum DequeueReply {
    /// One or more messages under the key were withdrawn: from the
    /// queue, or from the offered batch before the loop took it.
    Dequeued,
    /// Nothing under the key was withdrawn: none waited, or the loop
    /// had already taken the batch that held them.
    Empty,
    /// The agent's server could not be asked to give the batch back:
    /// its own words, or the call's.
    Error(Error),
}

impl From<DequeueReply> for dequeue::response::Frame {
    fn from(reply: DequeueReply) -> Self {
        match reply {
            DequeueReply::Dequeued => dequeue::response::Frame::Dequeued,
            DequeueReply::Empty => dequeue::response::Frame::Empty,
            DequeueReply::Error(error) => dequeue::response::Frame::Error(error),
        }
    }
}
