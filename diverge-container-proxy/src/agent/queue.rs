//! What the queue holds, and what the server's channels say to it.

use diverge_sdk::shared::containers::enqueue;
use diverge_sdk::shared::containers::enqueue::request::Message;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
use tokio::sync::oneshot;

/// What became of a batch, every message in it alike, as the server's
/// enqueue channel is answered.
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

/// One batch in the queue — one enqueue channel's messages — and
/// where its fate goes.
pub struct Batch {
    /// The batch's messages, in order, each under the caller's key,
    /// which its user parts carry back.
    pub messages: Vec<Message>,
    /// The enqueue channel waiting for the fate. A receiver that is
    /// gone is a server that left, and a fate nobody hears.
    pub fate: oneshot::Sender<Fate>,
}

/// What a server's channel asks of the driver.
pub enum Cmd {
    /// A batch of messages for the agent.
    Enqueue(Batch),
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
