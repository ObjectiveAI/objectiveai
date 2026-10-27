//! What a client's request frame carries for an image's info.

use serde::{Deserialize, Serialize};

use crate::shared::containers::request::Image;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask a provider for an image's `/DIVERGE.md`: the [`Image`] — its
/// digest, and the references under which its bytes may be fetched.
///
/// The provider finds the image as a
/// [run](crate::provider::endpoints::containers::agents::run) finds
/// it — the caller's own images first, then what it holds, then the
/// referenced registries its policy lists — and reads the file at the
/// image's root without running anything. The image a run of the same
/// request would start is the image whose file is answered.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The image asked about.
    pub image: Image,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. See the table in
/// [`endpoints`](crate::provider::endpoints) for the whole allocation.
const TAG: u8 = 3;

impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        serde_json::from_slice(rest).map_err(FrameError::Body)
    }
}

/// An image info request frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => f.write_str("an image info request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "expected the image info request tag {TAG}, found {tag}"),
            FrameError::Body(error) => write!(f, "an image info request did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
