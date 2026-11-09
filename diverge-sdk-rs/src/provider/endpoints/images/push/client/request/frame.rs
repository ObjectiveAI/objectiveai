//! What a client's request frame carries for an image push.

use serde::{Deserialize, Serialize};

use crate::shared::containers::request::Image;
use crate::shared::description::Description;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Push an image into the caller's Store on the provider: the
/// [`Image`] — its digest, and the references under which its bytes
/// may be fetched, the same a run names — and what the caller says of
/// it.
///
/// # The digest is the image
///
/// What the caller comes to own is the digest, and the bytes the
/// provider holds under it, verified. The references are only
/// locators: a provider fetches from one whose registry its policy
/// lists, and ignores the rest; a push with none takes the image from
/// the caller, or not at all. See
/// [`push`](crate::provider::endpoints::images::push) for the order.
///
/// # What a push answers
///
/// Zero or more
/// [`Blob`](crate::provider::endpoints::images::push::server::response::Frame::Blob)
/// answers, one per blob pulled from the caller, then exactly one of
/// [`Pushed`](crate::provider::endpoints::images::push::server::response::Frame::Pushed),
/// [`Exists`](crate::provider::endpoints::images::push::server::response::Frame::Exists),
/// [`InsufficientCapacity`](crate::provider::endpoints::images::push::server::response::Frame::InsufficientCapacity),
/// [`Unavailable`](crate::provider::endpoints::images::push::server::response::Frame::Unavailable),
/// [`ProofFailed`](crate::provider::endpoints::images::push::server::response::Frame::ProofFailed),
/// [`Malformed`](crate::provider::endpoints::images::push::server::response::Frame::Malformed),
/// [`Unsupported`](crate::provider::endpoints::images::push::server::response::Frame::Unsupported)
/// and [`Error`](crate::provider::endpoints::images::push::server::response::Frame::Error),
/// then the finish.
///
/// # On the wire
///
/// JSON after the tag, as [`images::oci`](crate::provider::endpoints::images::oci)
/// and the runs carry an [`Image`]: its references are absent when
/// there are none, which a format with no self-description cannot
/// say.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The image to push.
    pub image: Image,
    /// What the caller says of the image, at most 132 characters;
    /// absent for nothing. Not written over an image the caller owns
    /// already, which is `Exists`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
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
const TAG: u8 = 18;

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

/// An image push request that could not be read.
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
            FrameError::Empty => f.write_str("image push request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected image push request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => write!(f, "image push request did not parse: {error}"),
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
