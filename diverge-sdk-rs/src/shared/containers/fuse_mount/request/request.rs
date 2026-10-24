//! Where to mount, under what id, and which kind.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::fuse::Kind;

/// Serve one more file or directory into the running container.
///
/// Opened on the run's scope at any time after the id. The mount is
/// what a [`FuseMount`](crate::shared::containers::request::FuseMount)
/// of the run's request is — the same path rules, the same asks under
/// the same id — with the kind said here, since there is no list to
/// put it on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// Where the mount appears inside the container, as path
    /// components from its root: not the root, no component empty,
    /// `.` or `..`, and no mount's the container holds, inside none
    /// and holding none.
    pub container_path: Vec<String>,
    /// The caller's own id for the mount, echoed back on every ask the
    /// provider sends for it: not one the container holds a FUSE
    /// mount under.
    pub id: String,
    /// One file, or a tree. See [`Kind`].
    pub kind: Kind,
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
