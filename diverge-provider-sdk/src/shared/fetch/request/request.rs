//! Asking for content by its identity.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use super::Kind;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The kind and the hash, and deliberately nothing else.
///
/// Not the name: a name is the caller's label and may point at
/// different content tomorrow, where the dirhash is the content. The
/// client answers with the directory's files — one
/// [`response::Frame`](super::super::response::Frame) each — or, if
/// it does not hold the hash, with the empty finish.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// What the content is, which is which folder it lives in.
    pub kind: Kind,
    /// The content's deterministic identity.
    pub dirhash: String,
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
