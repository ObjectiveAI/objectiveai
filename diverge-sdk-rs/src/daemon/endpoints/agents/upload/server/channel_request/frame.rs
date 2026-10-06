//! What a server's channel request frame carries for an upload.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Send the content of one file.
///
/// What the daemon asks the client for, and the only thing: the bytes
/// travel as responses on a channel the daemon opened — one per file of
/// a directory, one for a file — and the client answers with the file's
/// pieces and finishes. A struct and no tag, as the resource upload's
/// ask is: one thing to ask for is a struct, and the scope says which
/// upload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// Which file: its path as the request named it, for a directory;
    /// absent for a file, which has the one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// JSON, and nothing in front of it: the payload is the object.
impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
