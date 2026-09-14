//! Asking for a directory by its identity.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The identity, and deliberately nothing else.
///
/// Not the mount path: a path is the caller's placement and may
/// point at different content tomorrow, where the identity IS the
/// content. The client answers with the directory's files — one
/// [`fetch_directory::Frame`](crate::shared::containers::fetch_directory::response::Frame)
/// per file, chunked adjacently where a file is large — or, if it
/// does not hold the identity, with the empty finish.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// The directory's size-bearing identity:
    /// `<total size>:<h1 dirhash>` — Go's directory hash, as the
    /// volumes stat's
    /// [`dirhash`](crate::endpoints::volumes::stat::server::response::Stat::dirhash)
    /// states it, and the sum of the lengths of the files it hashed.
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
