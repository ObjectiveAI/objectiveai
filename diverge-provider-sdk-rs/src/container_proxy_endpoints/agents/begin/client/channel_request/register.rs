//! The agent, as the request that made the container carried it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The agent, handed to the container once: the
/// [`agent`](crate::endpoints::containers::agents::run::client::request::Frame::agent)
/// of the request that made it, typed to the same depth for the same
/// reason — a JSON value, because the image defines what an agent
/// is, and what the value may be is what
/// [`agent_schema`](crate::shared::containers::agent_schema) answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Register {
    /// The agent, as the image defines it.
    pub agent: Value,
}

impl Encode for Register {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Register {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
