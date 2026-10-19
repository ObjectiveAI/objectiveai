//! What a client's request frame carries for an image build.

use serde::{Deserialize, Serialize};

use super::Context;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Build an image on the provider, from a Containerfile, under a
/// name.
///
/// # The name is the caller's, and has to stay unique
///
/// Nothing derives it from anything and nothing constrains it here.
/// What a provider will accept as a name is the provider's to state,
/// and a caller learns it by being refused. A name the caller already
/// has an image by is refused; a caller that wants the name again
/// deletes the image first.
///
/// # The Containerfile is the whole recipe
///
/// Its text, verbatim, as the provider's build tool reads one. What
/// it may `FROM` is what the provider can obtain, by its own policy;
/// what it may `COPY` is the context, which is the volume directory
/// [`context`](Self::context) names, or nothing. A build reads its
/// context and writes nothing into it: a Containerfile copies in and
/// never out.
///
/// # What a build answers
///
/// The provider's build tool says things as it goes, and the caller
/// hears them: zero or more
/// [`Output`](crate::provider::endpoints::images::build::server::response::Frame::Output)
/// lines, then exactly one of
/// [`Built`](crate::provider::endpoints::images::build::server::response::Frame::Built),
/// [`InsufficientCapacity`](crate::provider::endpoints::images::build::server::response::Frame::InsufficientCapacity)
/// and [`Error`](crate::provider::endpoints::images::build::server::response::Frame::Error).
/// A build that fails says why in its output, and the error after it
/// says only that it failed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame {
    /// What to call the image, once built.
    pub name: String,
    /// The Containerfile, verbatim.
    pub containerfile: String,
    /// The directory of one of the caller's volumes that is the
    /// build's context and working directory, or none: see
    /// [`Context`]. With none, the build has no context, and a
    /// Containerfile that copies from one fails. One postcard byte,
    /// `0` for none and `1` followed by the context.
    pub context: Option<Context>,
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
const TAG: u8 = 17;

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

/// An image build request that could not be read.
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
            FrameError::Empty => f.write_str("image build request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected image build request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => write!(f, "image build request did not parse: {error}"),
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
