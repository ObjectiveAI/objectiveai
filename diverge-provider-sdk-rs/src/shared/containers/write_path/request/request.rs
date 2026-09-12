//! One file to write.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Write one file into the container.
///
/// Carries no content. This opens the exchange, names its destination
/// and labels it; the provider answers by asking for the bytes on a
/// channel of its own — see
/// [`write_bytes`](crate::shared::containers::write_bytes).
///
/// No offset and no length. A write replaces whatever is at the path,
/// whole, and a length stated here would be a promise about a file the
/// sender may still be reading.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// What this write is called, chosen by whoever asked for it.
    ///
    /// The provider quotes it back in the
    /// [`write_bytes::request::Request`](crate::shared::containers::write_bytes::request::Request)
    /// that asks for the content, and that is the whole of the
    /// correlation: a client with several writes in flight learns
    /// which one is being asked about.
    ///
    /// # It is the client's to choose and the client's to keep unique
    ///
    /// Unique among the writes this client has open — reusing one that
    /// is still outstanding makes two asks indistinguishable, and the
    /// client is the only party that could have prevented it. A number
    /// that counts up is the obvious way and nothing requires it.
    ///
    /// Reusing one after a write has finished is fine. Nothing here
    /// remembers.
    ///
    /// # Why not the channel it arrived on
    ///
    /// Because channels are numbered per SENDER. The client opened
    /// this write on a channel of its own, the provider asks for the
    /// content on a channel of its own, and neither side's header can
    /// name the other's — so a payload quoting a channel number would
    /// be quoting one out of a namespace its reader does not share.
    ///
    /// An id the client invents belongs to the write rather than to
    /// either channel, which is what makes it readable on both sides.
    pub write_id: u32,
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
