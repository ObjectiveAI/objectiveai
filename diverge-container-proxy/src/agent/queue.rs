//! What the queue holds, and what the server's channels say to it.

use diverge_sdk::shared::containers::enqueue;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
use rmcp::model::ContentBlock;
use tokio::sync::oneshot;

/// What became of a message, as the server's enqueue channel is
/// answered.
pub enum Fate {
    /// The agent's program has it: the `/run` it started answered
    /// `2xx`, or the `/enqueue` that carried it to the loop in flight
    /// answered `204`.
    Delivered,
    /// The agent's server refused it, or no loop could start on it:
    /// its own words.
    Error(Error),
}

impl From<Fate> for enqueue::response::Frame {
    fn from(fate: Fate) -> Self {
        match fate {
            Fate::Delivered => enqueue::response::Frame::Delivered,
            Fate::Error(error) => enqueue::response::Frame::Error(error),
        }
    }
}

/// One message in the queue, and where its fate goes.
pub struct Queued {
    /// The caller's key, which its user parts carry back.
    pub key: String,
    /// The daemon's arrival order, lower first: where the message
    /// stands in the queue.
    pub index: u64,
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
