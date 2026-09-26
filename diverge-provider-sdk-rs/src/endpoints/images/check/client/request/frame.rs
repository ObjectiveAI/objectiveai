//! What a client's request frame carries for an image check.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Ask a provider whether it can supply a particular image.
///
/// No registry. The digest is the image's identity and a registry is
/// only a locator — any registry serving these bytes serves the same
/// image, because a client recomputes the hash on pull and rejects a
/// mismatch. So WHERE a provider gets it is the provider's business:
/// its own mirror, a pull-through cache, a private registry it has
/// credentials for, or something it already holds locally.
///
/// That is also what makes proprietary images answerable. A provider
/// with access to an image no public registry serves can still say
/// yes, and a caller naming a registry it cannot reach would be
/// asserting something it has no standing to assert.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame {
    /// The repository path — `library/nginx`, `myorg/myimage`.
    ///
    /// Kept alongside the digest because a digest alone is not
    /// resolvable: every registry API is repository-scoped, and there
    /// is no lookup from a digest to wherever it lives.
    pub name: String,
    /// The manifest digest, `<algorithm>:<hex>`.
    ///
    /// What actually identifies the image. Unlike a tag, it cannot be
    /// repointed at different content.
    pub digest: String,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 14;

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

/// A an image check request request frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    ///
    /// A reader that dispatched on the tag will not see this. One that
    /// assumed which request it held, and was wrong, will — which is
    /// the point of checking a tag rather than skipping it.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("an image check request request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected an image check request request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "an image check request request did not parse: {error}")
            }
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
