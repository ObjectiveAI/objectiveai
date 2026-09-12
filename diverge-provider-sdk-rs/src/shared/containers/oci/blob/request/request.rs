//! Asking for a blob by its digest.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The digest, and deliberately nothing else.
///
/// No offset and no length: the provider fetches a blob whole, once,
/// and serves every range out of its own store, because a partial
/// blob is a blob it cannot verify. No name, because the store is
/// keyed by digest and a blob under one name is the same blob under
/// another.
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
