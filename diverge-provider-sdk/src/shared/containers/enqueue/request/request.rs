//! A message for the running loop's queue.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Enqueue a message into the running loop.
///
/// The verb is the mechanism's: the message is QUEUED, not injected —
/// the turn in flight always runs to completion, and the agent picks
/// the message up at a seam of its own choosing: folded in beside the
/// next tool results, or opening the next turn when the assistant has
/// already finished. Nothing about this interrupts anything, ever.
///
/// # The content is a string
///
/// Plain text, deliberately: a mid-run steer is text. The
/// [`UserChunk`](crate::shared::containers::agentic_loop::response::UserChunk)
/// that marks this message's delivery carries the same string back,
/// verbatim, at the position it landed.
///
/// # The answer says what became of it
///
/// One frame, then the finish: `delivered` when the agent has taken
/// the message into the conversation, `dequeued` when the caller
/// withdrew it first, `missed` when the run ended — or none was
/// running — before it could be taken, and an error for everything
/// else. The first three carry nothing — the fate is the answer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// The message's text.
    pub prompt: String,
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
