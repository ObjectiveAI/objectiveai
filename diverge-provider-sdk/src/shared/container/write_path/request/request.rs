//! One file to write.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Write one file into the container.
///
/// Carries no content. This opens the exchange and names its
/// destination; the provider answers by asking for the bytes on a
/// channel of its own — see
/// [`write_bytes`](crate::shared::container::write_bytes).
///
/// No offset and no length. A write replaces whatever is at the path,
/// whole, and a length stated here would be a promise about a file the
/// sender may still be reading.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// The destination, as path components from the container's root.
    ///
    /// The same meaning of "path" as everywhere else in this API, and
    /// the same frame of reference a
    /// [`filetree`](crate::shared::filetree) stream uses.
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
