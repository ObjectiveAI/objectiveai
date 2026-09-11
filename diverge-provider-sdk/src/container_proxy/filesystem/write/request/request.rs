//! The destination.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The file to write, the first message on `/filesystem/write`.
///
/// No length and no mode: the content follows as
/// [`Frame`](super::Frame)s until the empty one, and what the file
/// looks like on disk is the container's business.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// The destination, as path components from the container's
    /// root — the same meaning of "path" as everywhere else in this
    /// crate, and the same frame of reference a
    /// [`filetree`](crate::shared::filetree) stream uses. Components
    /// rather than a joined string: a path is a sequence, and joining
    /// it would invent a separator that then has to be escaped out
    /// of names containing it.
    pub path: Vec<String>,
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
