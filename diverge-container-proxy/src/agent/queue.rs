//! What the queue holds, and what the server's channels say to it.

use diverge_sdk::shared::containers::enqueue;
use diverge_sdk::shared::containers::enqueue::request::Item;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
use tokio::sync::oneshot;

/// What became of a batch, every message in it alike, as the server's
/// enqueue channel is answered.
pub enum Fate {
    /// The agent's program has it: the `/run` it started answered
    /// `2xx`, or the `/enqueue` that carried it to the loop in flight
    /// answered `204`. A structured decision is only ever the first:
    /// the `/run` it alone started answered `2xx`.
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

/// One batch in the queue — what one enqueue channel carried: chat
/// messages, or one structured decision — and where its fate goes.
pub struct Batch {
    /// What the channel carried: a batch of chat messages, in order,
    /// each under the caller's key, which its user parts carry back;
    /// or one structured decision, under its key.
    pub item: Item,
    /// The enqueue channel waiting for the fate. A receiver that is
    /// gone is a server that left, and a fate nobody hears.
    pub fate: oneshot::Sender<Fate>,
}

impl Batch {
    /// Whether the batch is a structured decision: what is handed
    /// over alone, to a program that is idle.
    pub fn is_decision(&self) -> bool {
        matches!(self.item, Item::StructuredDecisions(_))
    }
}

/// What a server's channel asks of the driver.
pub enum Cmd {
    /// A batch for the agent, and its index among the server's
    /// enqueues: the order the driver queues the batches in, whatever
    /// order they reach it in.
    Enqueue(u64, Batch),
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
