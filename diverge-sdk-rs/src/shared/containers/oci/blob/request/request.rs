//! Asking for a blob by its digest.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The digest, and deliberately nothing else.
///
/// No offset and no length: the provider asks for the blob, and
/// what it does with the bytes — holds them, streams them through —
/// is its own, so long as what it serves is the digest's. No name,
/// because a blob is identified by its digest and a blob under one
/// name is the same blob under another.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// The blob's digest, `<algorithm>:<hex>`.
    pub digest: String,
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
