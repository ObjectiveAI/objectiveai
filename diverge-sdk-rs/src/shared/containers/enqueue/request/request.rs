//! A message for the running loop's queue.

use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Send a message to the agent.
///
/// With no loop running, the message starts one, and is its input.
/// With one running, the verb is the mechanism's: the message is QUEUED, not injected —
/// the turn in flight always runs to completion, and the agent picks
/// the message up at a seam of its own choosing: folded in beside the
/// next tool results, or opening the next turn when the assistant has
/// already finished. Nothing about this interrupts anything, ever.
///
/// # The content is MCP's
///
/// A list of content blocks — text, an image, audio, an embedded
/// resource, a link to one — in order, each as MCP's own
/// [`ContentBlock`] states it: one vocabulary for what a caller says
/// to an agent, what a tool answers, and what the agent says back.
/// What an agent's image makes of a block is its own. An image that
/// cannot take one refuses the whole message, in its own words, and
/// the fate carries them; nothing is reduced to its text on the
/// caller's behalf. A message with no block is refused too. The
/// [user parts](crate::provider::endpoints::containers::agents::run::server::response::user_parts)
/// that mark this message's delivery carry the same blocks back, one
/// chunk each, verbatim, under this key, at the position it landed.
///
/// # The answer says what became of it
///
/// One frame, then the finish: `delivered` when the agent's program
/// has the message — the loop it started, or the program's own
/// queue, taken at its next seam — and an error when the agent
/// refused the message or no run could start on it. A run ending
/// with the message still waiting does not lose it: the message
/// starts the next run. `delivered` carries nothing — the fate is the
/// answer — and nothing withdraws a message once sent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// The caller's handle on the message: what each of its user
    /// parts carries back, so the log credits its sender. Not unique —
    /// two messages may share one — and not read by the provider or
    /// the proxy, which carry it and nothing else.
    pub key: String,
    /// The message's content, in order.
    pub content: Vec<ContentBlock>,
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
