//! One file to move.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Copy a file out of this container into another.
///
/// # The source is the container you are attached to
///
/// There is no source id, and that is the access model rather than a
/// convenience. A transfer is asked FOR on a scope, and the scope
/// already names a container — so a caller moves files out of the one
/// it holds and cannot name two it does not.
///
/// The destination is the other way round, and worth being clear
/// about: [`destination_id`](Self::destination_id) is a claim, not a
/// proof. Being attached to the source says nothing about the right to
/// write into somewhere else, so a provider decides that separately,
/// and a caller that guesses an id it has no business touching learns
/// so by being refused.
///
/// See [`transfer`](super::super) for when this works at all and what
/// to do when it does not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// The file to copy, as path components from THIS container's
    /// root.
    ///
    /// The same meaning of "path" as everywhere else in this API, and
    /// the same frame of reference a
    /// [`filetree`](crate::shared::filetree) stream uses — so a caller
    /// watching a container transfers a file by handing back the path
    /// the watch just named.
    pub path: Vec<String>,
    /// Which container to copy it into.
    ///
    /// An [`Id`](crate::endpoints::laboratories::run::server::response::Frame::Id)
    /// from a run. It may be this container, which makes the
    /// transfer a copy within one filesystem and is the case a
    /// provider can do most cheaply of all.
    pub destination_id: String,
    /// Where in it, as path components from the DESTINATION
    /// container's root.
    ///
    /// Whatever is there is replaced, whole, the way a
    /// [`write_path`](crate::shared::container::write_path) replaces
    /// it.
    pub destination_path: Vec<String>,
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
