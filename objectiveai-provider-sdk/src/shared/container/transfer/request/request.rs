//! One file to move.

use serde::{Deserialize, Serialize};

use super::Location;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Copy a file from one container to another.
///
/// Both ends named, no content anywhere. See
/// [`transfer`](super::super) for when this works and what to do when
/// it does not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Request {
    /// The file to copy.
    pub source: Location,
    /// Where to put it. Whatever is there is replaced, whole, the way
    /// a [`write_path`](crate::shared::container::write_path) replaces
    /// it.
    pub destination: Location,
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
