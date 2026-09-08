//! What the loop runs on.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The prompt and the agent, as the agent container's
/// [`request`](crate::endpoints::containers::agents::run::client::request::Frame)
/// carried them: what the server hands the loop, once, as its first
/// message.
///
/// The same two fields, typed to the same depths and for the same
/// reasons — the prompt is every loop's, the agent is the image's —
/// carried on to the program that will read them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Request {
    /// What the loop is asked, post-transform.
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
