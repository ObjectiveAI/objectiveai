//! One file to read.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Read one file out of the container.
///
/// No offset and no length. A read starts at the beginning and runs to
/// whatever end it finds, because a file being written to has no
/// stable size to address into — see
/// [`response::Frame`](super::super::response::Frame) for what that
/// costs and how it is reported.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// The file, as path components from the container's root.
    ///
    /// The same meaning of "path" as everywhere else in this API, and
    /// the same frame of reference a
    /// [`filetree`](crate::shared::filetree) stream uses — so a caller
    /// watching a container reads a file by handing back the path the
    /// watch just named.
    ///
    /// Components rather than a joined string: a path is a sequence,
    /// and joining it would invent a separator that then has to be
    /// escaped out of names containing it.
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
