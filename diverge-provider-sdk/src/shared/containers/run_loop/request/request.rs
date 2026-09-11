//! What one loop is asked.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Run one loop on this prompt.
///
/// The agent is not here: it was on the request that made the
/// container, registered once, and it never changes. The prompt is
/// each loop's, because a container runs loops one after another —
/// each resuming the conversation the last one left — and every one
/// is asked something.
///
/// POST-TRANSFORM: the result of whatever built the request — a
/// system prompt applied, a history folded in — so a provider never
/// rewrites what it was given.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// What the loop is asked.
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
    /// The ordinary JSON failure.
    type Error = Error;

    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes)
    }
}
