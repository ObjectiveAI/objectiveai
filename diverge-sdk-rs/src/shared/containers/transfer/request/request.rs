//! What to move, and where.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Copy one file out of this container into another.
///
/// Opened on the scope of the container the file is IN. The other
/// container is named by its id, and the client must be running or
/// connected to it — see [`transfer`](super::super) for the rule.
///
/// No `write_id`, unlike a [`write`](crate::shared::containers::write_path):
/// nothing is asked of the client, so there is nothing to correlate.
/// No offset and no length, as with a read and a write: the file is
/// copied whole, and the destination is replaced whole.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// The file, as path components from this container's root — the
    /// same meaning of "path" a [`read`](crate::shared::containers::read)
    /// carries, and the same frame of reference a
    /// [`filetree`](crate::shared::filetree) stream uses.
    pub path: Vec<String>,
    /// The container the file is written into, by the id its run
    /// answered — the [`Id`](crate::shared::containers::response::Id)
    /// a [`connect`](crate::shared::containers::request::Connect)
    /// names a container by.
    pub id: String,
    /// The destination, as path components from THAT container's
    /// root, replaced whole, as a
    /// [`write`](crate::shared::containers::write_path) replaces it.
    pub destination: Vec<String>,
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
