//! What the tree leaves out.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The first message on `/filesystem/tree`, from the server: the
/// paths the tree does not contain.
///
/// The server names the MOUNTS here — every one it placed in the
/// container, volume and identity and FUSE alike, whose watch would
/// cost the walk and report what the caller already holds. `/proc`,
/// `/sys` and `/dev` are the proxy's own and are never listed. Each
/// path is components from the container's root, the shape every
/// path in this crate takes; an empty one is dropped rather than read
/// as the root. An ignored path does not exist as far as the stream
/// is concerned: absent from the snapshot, never watched, an event
/// under it dropped.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// The paths to leave out, each as components from the root.
    pub ignore: Vec<Vec<String>>,
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
