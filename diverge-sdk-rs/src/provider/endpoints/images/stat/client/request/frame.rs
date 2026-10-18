//! What a client's request frame carries for an image stat.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Examine one image the caller built: what it was built from.
///
/// # A name, and the same access model as everything else
///
/// The one field is an
/// [`Image::name`](crate::provider::endpoints::images::list::server::response::Image::name)
/// from a listing. A caller cannot examine an image it did not build,
/// and cannot probe for what exists by asking and reading the error —
/// the provider resolves a name against the caller that built it, and
/// nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame {
    /// Which image, by the name its build gave it.
    ///
    /// Names come from
    /// [`Image::name`](crate::provider::endpoints::images::list::server::response::Image::name)
    /// and mean nothing outside the caller that chose them.
    pub name: String,
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
const TAG: u8 = 16;

/// Postcard, matching [`volumes`](crate::provider::endpoints::volumes) and the
/// rest of this family's store.
impl Encode for Frame {
    /// Postcard's own failure. The tag cannot fail.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        postcard::to_io(self, &mut *out)?;
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
        postcard::from_bytes(rest).map_err(FrameError::Body)
    }
}

/// An image stat request that could not be read.
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
            FrameError::Empty => f.write_str("image stat request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected image stat request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => write!(f, "image stat request did not parse: {error}"),
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
