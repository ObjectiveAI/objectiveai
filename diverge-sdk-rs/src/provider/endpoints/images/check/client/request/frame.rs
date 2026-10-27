//! What a client's request frame carries for an image check.

use serde::{Deserialize, Serialize};

use super::Sources;
use crate::shared::containers::request::Image;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask a provider whether it can supply a particular image: the
/// [`Image`] — its digest, and the references under which its bytes
/// may be fetched — and where to look for it, the [`Sources`].
///
/// The digest is the image's identity and a reference is only a
/// locator — any registry serving these bytes serves the same image,
/// because a client recomputes the hash on pull and rejects a
/// mismatch. The references are where the client knows the bytes to
/// be, and a client that knows of none sends none; a provider asks a
/// referenced registry only where its own policy lists it.
///
/// # Where to look
///
/// Two places, each asked for or not, either or both: the client's
/// own images, built on this provider, and the referenced
/// registries. At least one is asked for, or the provider answers
/// [`NoSource`](crate::provider::endpoints::images::check::server::response::Frame::NoSource)
/// and determines nothing. Available means at least one source asked
/// has the image; the provider looks in the client's own images
/// before it asks a registry. What the provider holds on its own
/// account is not a place a check can look: no image it pulled is
/// revealed by one. See [`Sources`].
///
/// # An image the client built is one the provider holds, for it
///
/// A digest the client [built](crate::provider::endpoints::images::build)
/// on this provider is available to that client, from the store,
/// when the `client` source is asked; to any other client it is not
/// there, unless that client built it too. A provider never answers
/// one identity's store to another.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The image asked about.
    pub image: Image,
    /// Where to look: two booleans, flat in the JSON beside
    /// `image`.
    #[serde(flatten)]
    pub sources: Sources,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::provider::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 15;

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
