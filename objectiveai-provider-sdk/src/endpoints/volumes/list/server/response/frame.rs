//! What a server's response frame carries for a volume listing.

use serde::{Deserialize, Serialize};

use super::Directory;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Every directory a provider will let this caller watch.
///
/// The whole answer, in one frame. A listing is not a stream: a
/// provider knows what it offers before it is asked, so there is
/// nothing to discover incrementally and nothing to hold a channel
/// open for.
///
/// An empty list is a valid answer and means the provider offers
/// nothing — distinct from a failure to ask, which never produces a
/// frame at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame(
    /// The directories, in whatever order the provider chose. Nothing
    /// promises an order and nothing should be read into one.
    pub Vec<Directory>,
);

/// Postcard, matching [`filetree`](crate::shared::filetree) rather than the
/// JSON the rest of the crate uses.
///
/// The same reasoning: this relays nothing, so no byte of it has to
/// survive a round trip unchanged, and nothing downstream reads it as
/// text. It is also the same DATA — directories and component paths —
/// and encoding the two sides of one feature differently would be a
/// difference with nothing behind it.
impl Encode for Frame {
    /// Postcard's own failure.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        postcard::to_io(self, &mut *out)?;
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Postcard's own failure.
    type Error = postcard::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        postcard::from_bytes(bytes)
    }
}
