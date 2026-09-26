//! Asking whether an image is held.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The image, as the run request named it: the digest is what is
/// asked about, and the name rides with it for a store that keys by
/// both.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// The repository path, as the run request has it.
    pub name: String,
    /// The manifest digest, `<algorithm>:<hex>`.
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
