//! A batch of messages for the agent.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use super::Message;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Send a batch of messages to the agent: one or more, in order.
///
/// With no loop running, the batch starts one, and is its input.
/// With one running, the verb is the mechanism's: the batch is QUEUED, not injected —
/// the turn in flight always runs to completion, and the agent picks
/// the batch up at a seam of its own choosing: folded in beside the
/// next tool results, or opening the next turn when the assistant has
/// already finished. Nothing about this interrupts anything, ever.
///
/// # The content is MCP's
///
/// Each message is a list of content blocks — text, an image, audio,
/// an embedded resource, a link to one — in order, each as MCP's own
/// [`ContentBlock`](rmcp::model::ContentBlock) states it: one
/// vocabulary for what a caller says to an agent, what a tool answers,
/// and what the agent says back.
/// What an agent's image makes of a block is its own. An image that
/// cannot take one refuses the whole batch, in its own words, and
/// the fate carries them; nothing is reduced to its text on the
/// caller's behalf. A message with no block is refused too. The
/// [user parts](crate::provider::endpoints::containers::agents::run::server::response::user_parts)
/// that mark a message's delivery carry the same blocks back, one
/// chunk each, verbatim, under its key, at the position it landed.
///
/// # The answer says what became of it
///
/// One frame, then the finish, for the whole batch: `delivered` when
/// the agent's program has every message of it — the loop they
/// started, or the program's own queue, taken at its next seam — and
/// an error when the agent refused the batch or no run could start on
/// it. A run ending with the batch still waiting does not lose it:
/// the batch starts the next run. `delivered` carries nothing — the
/// fate is the answer — and nothing withdraws a batch once sent.
///
/// # The order is the batch's
///
/// The messages are in the order the caller means the agent to take
/// them, and nothing reorders them: the provider carries the batch
/// whole and does not read it, the proxy hands it to the agent's
/// program whole — in the `/run`, or in one `/enqueue` — and the
/// program joins its messages in that order, as one user turn. A
/// caller that sends one batch at a time, each after the last was
/// answered, has its messages taken in the order it sent them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// The batch's messages, in order, each under its key.
    pub messages: Vec<Message>,
}

/// Its JSON, and nothing in front of it. The tag that says which
/// request this is belongs to whichever frame carries it.
impl Encode for Request {
    /// The ordinary JSON failure.
    type Error = Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Request {
    /// The ordinary JSON failure. There is nothing else here to get
    /// wrong — no tag to be unknown, and no empty case, since no bytes
    /// at all is a JSON document that ended too early and is reported
    /// as one.
    type Error = Error;

    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes)
    }
}
