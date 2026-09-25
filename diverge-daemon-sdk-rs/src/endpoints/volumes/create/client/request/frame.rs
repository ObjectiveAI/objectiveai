//! What a client's request frame carries for a volume create.

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};

/// The provider's own request, verbatim, behind the daemon's tag.
///
/// The fields are the provider protocol's and are documented there —
/// [`diverge_provider_sdk::endpoints::volumes::create::client::request::Frame`] — and mean the same here,
/// the volume being the daemon's. Postcard after the tag, as the
/// provider's is: the bytes after the tag byte are the same bytes a
/// client would send a provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame(pub diverge_provider_sdk::endpoints::volumes::create::client::request::Frame);

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](diverge_provider_sdk::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 11;

/// Postcard, as the provider's is: the tag, and the inner request's
/// own bytes.
impl Encode for Frame {
    /// Postcard's own failure. The tag cannot fail.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        postcard::to_io(&self.0, &mut *out)?;
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is postcard's.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        postcard::from_bytes(rest).map(Frame).map_err(FrameError::Body)
    }
}

/// A volume create request that could not be read.
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
    Body(postcard::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => f.write_str("volume create request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected volume create request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "volume create request did not parse: {error}")
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
