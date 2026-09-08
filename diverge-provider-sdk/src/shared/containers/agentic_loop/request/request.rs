//! The loop's request.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// What to run the loop on: a prompt, and the agent that runs it.
///
/// Two fields, and they are typed to different depths on purpose.
/// The prompt is a string, because every loop takes one and this
/// crate can say so. The agent is a JSON value, because this crate
/// does not know what an agent is — a model, a set of tools, a
/// personality, a harness's own knobs — and a wire that typed it
/// would have to be revised for every agent that ever ran. What the
/// value MAY be is what
/// [`agent_schema`](crate::shared::containers::agent_schema) answers,
/// so a caller learns an image's agent from the image rather than
/// from here.
///
/// The typed agents this crate once carried are kept in
/// [`agent`](crate::endpoints::containers::agents::agent) for
/// reference; nothing here reads them.
///
/// # JSON, and only JSON
///
/// A [`Value`] deserializes through `deserialize_any`, which a format
/// with no self-description cannot answer — the same reason
/// [`shared::error::Error`](crate::shared::error::Error) is JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Request {
    /// What the loop is asked. POST-TRANSFORM: the result of whatever
    /// built the request — a system prompt applied, a history folded
    /// in — so a provider never rewrites what it was given.
    pub prompt: String,
    /// The agent, as the image defines it.
    pub agent: Value,
}

impl Encode for Request {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Request {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
