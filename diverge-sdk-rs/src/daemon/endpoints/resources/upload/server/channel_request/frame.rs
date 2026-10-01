//! What a server's channel request frame carries for an upload.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Send the content of one file.
///
/// What the daemon asks the client for, and the only thing: an
/// upload cannot carry its own content, because only a responder can
/// finish a channel, so the bytes travel as responses on a channel
/// the daemon opened — one per file of a directory, one for a file
/// resource. The client answers with the file's pieces and finishes.
///
/// # A struct, and no tag
///
/// One thing to ask for is a struct; an enum of one variant would be
/// a discriminant with nothing to discriminate, and a tag byte is
/// that discriminant written on the wire, so it goes for the same
/// reason. The scope IS the upload: which scope the ask arrived on
/// says which upload, and the path says which file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// Which file: its path as the request named it, for a directory;
    /// absent for a file resource, which has the one.
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
