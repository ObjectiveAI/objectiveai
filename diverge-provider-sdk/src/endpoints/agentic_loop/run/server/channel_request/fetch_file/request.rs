//! Asking for a file by its identity.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The identity, and deliberately nothing else.
///
/// Not the mount path: a path is the caller's placement and may
/// point at different content tomorrow, where the identity IS the
/// content. The client answers with the file's bytes — appending
/// [`fetch_file::Frame`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_file::Frame)s
/// — or, if it does not hold the identity, with the empty finish.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// The file's size-bearing identity:
    /// `f1:<size>:<base64url sha256 of the bytes>`.
    pub identity: String,
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
