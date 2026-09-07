//! The loop's request.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// What to run the loop on, as the image defines it.
///
/// A JSON value and nothing more, because this crate does not know
/// what an agent takes: a prompt, a conversation, a model, a set of
/// tools — that is the image's to define and the image's to change,
/// and a wire that typed it would have to be revised for every agent
/// that ever ran. What the value MAY be is what
/// [`schema`](crate::shared::containers::schema) answers, so a caller
/// learns an image's request from the image rather than from here.
///
/// The typed configurations this crate once carried are kept in
/// [`agent`](crate::endpoints::containers::agents::agent) for
/// reference; nothing here reads them.
///
/// # JSON, and only JSON
///
/// A [`Value`] deserializes through `deserialize_any`, which a format
/// with no self-description cannot answer — the same reason
/// [`shared::error::Error`](crate::shared::error::Error) is JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Request(
    /// The request, as the image defines it.
    pub Value,
);

impl Encode for Request {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, &self.0)
    }
}

impl Decode<'_> for Request {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes).map(Request)
    }
}
