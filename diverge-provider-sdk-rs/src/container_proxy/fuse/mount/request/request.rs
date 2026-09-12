//! The mount to make.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::fuse::Kind;

/// The first message on `/fuse/mount`, from the server: one FUSE
/// mount, as the container request named it.
///
/// One
/// [`FuseMount`](crate::shared::containers::request::FuseMount) of
/// the request, with which list it was on made explicit as the
/// [`kind`](Self::kind). The path is components from the container's
/// root, never empty, no component empty or `.` or `..`; the id is
/// the caller's, echoed on every ask the mount makes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// Where the mount goes, as components from the container's
    /// root.
    pub path: Vec<String>,
    /// The mount's id, the caller's.
    pub id: String,
    /// One regular file, or a directory tree.
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
